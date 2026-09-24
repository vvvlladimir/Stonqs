use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::plugins::reader::ReaderWarning;
use crate::state::AppState;
use chrono::Local;
use serde::Serialize;
use sq_core::import::{
    ImportMapping, ImportOptions, ImportPreview, ImportResult, ImportService, ParseConfig, PriceImport,
    PriceMapping, RowOverride, is_canonical, is_flex, parse_file,
};
use sq_core::storage::Store;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize)]
pub struct LoadedFile {
    pub name: String,
    pub size: usize,
    /// `<plugin id>/<reader id>` when a plugin's reader is what turned this file into something
    /// the wizard can read (ADR-0073). Absent for every file a shipped reader handled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader: Option<String>,
}

/// The file the wizard is working on. `content` is what every preview and the commit read, and
/// for a file a plugin claimed it is **what the reader produced**, not what was on disk: the
/// reader runs once, at load, and nothing calls it again (ADR-0073).
pub struct ImportFile {
    pub info: LoadedFile,
    pub content: Vec<u8>,
    pub warnings: Vec<ReaderWarning>,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewData {
    #[serde(flatten)]
    pub preview: ImportPreview,
    pub headers: Vec<String>,
    /// The layout the file was recognised as when it was loaded, so the wizard can say which
    /// one it applied. Only ever set by `import_load`: a later preview is the user's own doing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_template: Option<String>,
    /// `<plugin id>/<reader id>` when a plugin's reader produced what the wizard is showing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader: Option<String>,
    /// What the plugin's reader had to say about the file it read. Kept apart from the preview's
    /// own problems: these are a stranger's wording about a file the app never saw.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reader_warnings: Vec<ReaderWarning>,
}

#[tauri::command]
pub fn import_load(state: State<AppState>, name: String, content: Vec<u8>) -> UiResult<ImportPreviewData> {
    let size = content.len();
    // A plugin's reader gets the file after the two shipped formats that describe themselves and
    // before the CSV reader, which accepts nearly anything and would never let one through. What
    // it produces replaces the bytes: everything downstream reads the app's own transaction file,
    // so the preview and the commit cannot see different things (ADR-0073).
    let (content, reader, warnings) = if is_canonical(&content) || is_flex(&content) {
        (content, None, Vec::new())
    } else {
        match state.plugins.read_file(&name, &content)? {
            Some((id, reading)) => (reading.canonical.into_bytes(), Some(id), reading.warnings),
            None => (content, None, Vec::new()),
        }
    };

    // The file is recognised before anything is detected from it: a layout answers every
    // question the wizard is about to ask, and applying it is what "it just opens" means.
    let parsed = parse_file(&content, &ParseConfig::default())?;
    let head = String::from_utf8_lossy(&content[..content.len().min(2048)]).to_string();
    let found = crate::import_templates::match_for(
        &state.db_path()?,
        &state.plugins,
        &parsed.headers,
        Some(&name),
        &head,
    );

    *state.import_file()? = Some(ImportFile {
        // The name and the size are the file the user chose, not the document a reader made of it.
        info: LoadedFile {
            name,
            size,
            reader: reader.clone(),
        },
        content,
        warnings: warnings.clone(),
    });
    let (config, mapping) = match &found {
        Some(template) => (template.config.clone(), Some(template.mapping.clone())),
        None => (ParseConfig::default(), None),
    };
    let mut data = import_preview(state, config, mapping, Vec::new())?;
    // The id, not the name: a plugin's layout and a shipped one may print the same one.
    data.applied_template = found.map(|t| t.id);
    data.reader = reader;
    data.reader_warnings = warnings;
    Ok(data)
}

#[tauri::command]
pub fn import_load_path(state: State<AppState>, path: String) -> UiResult<ImportPreviewData> {
    let (name, content) = read_file(&path)?;
    import_load(state, name, content)
}

#[tauri::command]
pub fn import_prices_load_path(state: State<AppState>, path: String) -> UiResult<PriceImport> {
    let (name, content) = read_file(&path)?;
    import_prices_load(state, name, content)
}

fn read_file(path: &str) -> UiResult<(String, Vec<u8>)> {
    let content = std::fs::read(path).map_err(|e| UiError::invalid(format!("cannot read {path}: {e}")))?;
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    Ok((name, content))
}

#[tauri::command]
pub fn import_file_info(state: State<AppState>) -> UiResult<Option<LoadedFile>> {
    Ok(state.import_file()?.as_ref().map(|file| file.info.clone()))
}

#[tauri::command]
pub fn import_clear(state: State<AppState>) -> UiResult<()> {
    *state.import_file()? = None;
    Ok(())
}

#[tauri::command]
pub fn import_preview(
    state: State<AppState>,
    config: ParseConfig,
    mapping: Option<ImportMapping>,
    overrides: Vec<RowOverride>,
) -> UiResult<ImportPreviewData> {
    let file = state.import_file()?;
    let loaded = file
        .as_ref()
        .ok_or_else(|| UiError::invalid("no file is loaded"))?;
    let content = &loaded.content;
    let store = state.store()?;
    Ok(ImportPreviewData {
        headers: parse_file(content, &config)?.headers,
        preview: service(&store, &state)?.preview(content, &config, mapping.as_ref(), &overrides)?,
        applied_template: None,
        reader: loaded.info.reader.clone(),
        reader_warnings: loaded.warnings.clone(),
    })
}

fn service<'a>(store: &'a Store, state: &State<AppState>) -> UiResult<ImportService<'a>> {
    let base = state.portfolio()?.base_currency.clone();
    Ok(ImportService::new(store)
        .with_base_currency(&base)
        .as_of(Local::now().date_naive()))
}

#[tauri::command]
pub fn import_commit(
    app: AppHandle,
    state: State<AppState>,
    config: ParseConfig,
    mapping: Option<ImportMapping>,
    overrides: Vec<RowOverride>,
    options: ImportOptions,
) -> UiResult<ImportResult> {
    let result = {
        let file = state.import_file()?;
        let content = &file
            .as_ref()
            .ok_or_else(|| UiError::invalid("no file is loaded"))?
            .content;
        let store = state.store()?;
        let service = service(&store, &state)?;
        let preview = service.preview(content, &config, mapping.as_ref(), &overrides)?;
        service.commit(&preview, &options)?
    };

    crate::jobs::fetch_missing(&app, &state);
    emit_changed(&app, "transactions")?;
    Ok(result)
}

#[tauri::command]
pub fn import_prices_load(state: State<AppState>, name: String, content: Vec<u8>) -> UiResult<PriceImport> {
    let size = content.len();
    // A price file is read by the shipped readers alone: a plugin's reader produces operations,
    // and a price series is not one.
    *state.import_file()? = Some(ImportFile {
        info: LoadedFile {
            name,
            size,
            reader: None,
        },
        content,
        warnings: Vec::new(),
    });
    import_prices_preview(state, ParseConfig::default(), None)
}

#[tauri::command]
pub fn import_prices_preview(
    state: State<AppState>,
    config: ParseConfig,
    mapping: Option<PriceMapping>,
) -> UiResult<PriceImport> {
    let file = state.import_file()?;
    let content = &file
        .as_ref()
        .ok_or_else(|| UiError::invalid("no file is loaded"))?
        .content;
    let store = state.store()?;
    Ok(ImportService::new(&store).preview_prices(content, &config, mapping.as_ref())?)
}

#[tauri::command]
pub fn import_prices_commit(
    app: AppHandle,
    state: State<AppState>,
    config: ParseConfig,
    mapping: Option<PriceMapping>,
) -> UiResult<usize> {
    let saved = {
        let file = state.import_file()?;
        let content = &file
            .as_ref()
            .ok_or_else(|| UiError::invalid("no file is loaded"))?
            .content;
        let store = state.store()?;
        let service = ImportService::new(&store);
        let import = service.preview_prices(content, &config, mapping.as_ref())?;
        service.commit_prices(&import)?
    };

    emit_changed(&app, "quotes")?;
    Ok(saved)
}
