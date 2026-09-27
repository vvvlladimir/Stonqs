use super::off_thread;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::plugins::reader::{ReaderWarning, SkippedReader};
use crate::state::AppState;
use chrono::Local;
use serde::Serialize;
use sq_core::import::{
    ImportMapping, ImportOptions, ImportPreview, ImportResult, ImportService, ParseConfig, PriceImport,
    PriceMapping, RowOverride, is_canonical, is_flex, parse_file,
};
use sq_core::storage::Store;
use tauri::{AppHandle, Manager, State};

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
    /// Readers that broke over this file and were passed over on the way to what read it.
    pub skipped: Vec<SkippedReader>,
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
    /// Plugin readers that broke over the file and were passed over, so the file went on to the
    /// next reader — or to the app's own — instead of being refused.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_readers: Vec<SkippedReader>,
}

/// Off the main thread: a plugin's reader may run here, and it is allowed seconds.
#[tauri::command]
pub async fn import_load(app: AppHandle, name: String, content: Vec<u8>) -> UiResult<ImportPreviewData> {
    off_thread(move || load(app.state::<AppState>(), name, content)).await
}

fn load(state: State<AppState>, name: String, content: Vec<u8>) -> UiResult<ImportPreviewData> {
    let size = content.len();
    // A plugin's reader gets the file after the two shipped formats that describe themselves and
    // before the CSV reader, which accepts nearly anything and would never let one through. What
    // it produces replaces the bytes: everything downstream reads the app's own transaction file,
    // so the preview and the commit cannot see different things (ADR-0073).
    let (content, reader, warnings, skipped) = if is_canonical(&content) || is_flex(&content) {
        (content, None, Vec::new(), Vec::new())
    } else {
        let found = state.plugins.read_file(&name, &content)?;
        match found.read {
            Some((id, reading)) => (
                reading.canonical.into_bytes(),
                Some(id),
                reading.warnings,
                found.skipped,
            ),
            None => (content, None, Vec::new(), found.skipped),
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
        info: LoadedFile { name, size, reader },
        content,
        warnings,
        skipped,
    });
    let (config, mapping) = match &found {
        Some(template) => (template.config.clone(), Some(template.mapping.clone())),
        None => (ParseConfig::default(), None),
    };
    let mut data = import_preview(state, config, mapping, Vec::new())?;
    // The id, not the name: a plugin's layout and a shipped one may print the same one.
    data.applied_template = found.map(|t| t.id);
    Ok(data)
}

#[tauri::command]
pub async fn import_load_path(app: AppHandle, path: String) -> UiResult<ImportPreviewData> {
    off_thread(move || {
        let (name, content) = read_file(&path)?;
        load(app.state::<AppState>(), name, content)
    })
    .await
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
        skipped_readers: loaded.skipped.clone(),
    })
}

fn service<'a>(store: &'a Store, state: &State<AppState>) -> UiResult<ImportService<'a>> {
    let base = state.portfolio()?.base_currency.clone();
    Ok(ImportService::new(store)
        .with_base_currency(&base)
        .with_kind_dictionary(state.plugins.kind_words()?)
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
        skipped: Vec::new(),
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
