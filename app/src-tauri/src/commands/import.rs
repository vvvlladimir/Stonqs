use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use chrono::Local;
use serde::Serialize;
use sq_core::import::{
    ImportMapping, ImportOptions, ImportPreview, ImportResult, ImportService, ParseConfig, PriceImport,
    PriceMapping, RowOverride, parse_file,
};
use sq_core::storage::Store;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize)]
pub struct LoadedFile {
    pub name: String,
    pub size: usize,
}

#[derive(Debug, Serialize)]
pub struct ImportPreviewData {
    #[serde(flatten)]
    pub preview: ImportPreview,
    pub headers: Vec<String>,
}

#[tauri::command]
pub fn import_load(state: State<AppState>, name: String, content: Vec<u8>) -> UiResult<ImportPreviewData> {
    let size = content.len();
    *state.import_file()? = Some((LoadedFile { name, size }, content));
    import_preview(state, ParseConfig::default(), None, Vec::new())
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
    Ok(state.import_file()?.as_ref().map(|(info, _)| info.clone()))
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
    let (_, content) = file
        .as_ref()
        .ok_or_else(|| UiError::invalid("no file is loaded"))?;
    let store = state.store()?;
    Ok(ImportPreviewData {
        headers: parse_file(content, &config)?.headers,
        preview: service(&store, &state)?.preview(content, &config, mapping.as_ref(), &overrides)?,
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
        let (_, content) = file
            .as_ref()
            .ok_or_else(|| UiError::invalid("no file is loaded"))?;
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
    *state.import_file()? = Some((LoadedFile { name, size }, content));
    import_prices_preview(state, ParseConfig::default(), None)
}

#[tauri::command]
pub fn import_prices_preview(
    state: State<AppState>,
    config: ParseConfig,
    mapping: Option<PriceMapping>,
) -> UiResult<PriceImport> {
    let file = state.import_file()?;
    let (_, content) = file
        .as_ref()
        .ok_or_else(|| UiError::invalid("no file is loaded"))?;
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
        let (_, content) = file
            .as_ref()
            .ok_or_else(|| UiError::invalid("no file is loaded"))?;
        let store = state.store()?;
        let service = ImportService::new(&store);
        let import = service.preview_prices(content, &config, mapping.as_ref())?;
        service.commit_prices(&import)?
    };

    emit_changed(&app, "quotes")?;
    Ok(saved)
}
