use crate::commands::portfolio::require_text;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use serde::Deserialize;
use sq_core::import::{
    AttributeCsvConfig, AttributeImportResult, AttributePreview, ParseConfig, attributes_to_csv,
    build_attribute_preview, commit_attributes, detect_attribute_config, parse_csv,
};
use sq_core::prelude::*;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn attribute_defs_list(state: State<AppState>) -> UiResult<Vec<SecurityAttributeDef>> {
    Ok(state.store()?.list_attribute_defs()?)
}

#[derive(Debug, Deserialize)]
pub struct AttributeDefInput {
    pub id: Option<String>,
    pub name: String,
    pub kind: AttributeKind,
    pub unit: Option<String>,
    pub position: i64,
}

/// The kind is fixed once values exist behind it: changing it would either strand the values
/// it can no longer read or delete them silently, so an existing attribute keeps the kind it
/// was created with and only its name, unit and order are editable.
#[tauri::command]
pub fn attribute_def_save(
    app: AppHandle,
    state: State<AppState>,
    input: AttributeDefInput,
) -> UiResult<SecurityAttributeDef> {
    let name = require_text(&input.name, "name")?;

    let def = {
        let store = state.store()?;
        let mut def = match &input.id {
            Some(id) => store.get_attribute_def(id)?,
            None => SecurityAttributeDef::new(&name, input.kind),
        };
        def.name = name;
        def.unit = input.unit.map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
        def.position = input.position;
        store.save_attribute_def(&def)?;
        def
    };

    emit_changed(&app, "securities")?;
    Ok(def)
}

#[tauri::command]
pub fn attribute_def_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_attribute_def(&id)?;
    emit_changed(&app, "securities")
}

/// What a file would do, before it does it: which instruments it finds, which attributes it
/// would create, and which cells it refuses. Writes nothing.
#[tauri::command]
pub fn attributes_import_preview(
    state: State<AppState>,
    content: Vec<u8>,
    config: Option<AttributeCsvConfig>,
) -> UiResult<AttributePreview> {
    let store = state.store()?;
    let parsed = parse_csv(&content, &ParseConfig::default())?;
    let config = config.unwrap_or_else(|| detect_attribute_config(&parsed));
    Ok(build_attribute_preview(
        &parsed,
        &config,
        &store.list_securities()?,
        &store.list_attribute_defs()?,
    ))
}

#[tauri::command]
pub fn attributes_import_preview_path(
    state: State<AppState>,
    path: String,
    config: Option<AttributeCsvConfig>,
) -> UiResult<AttributePreview> {
    let content = std::fs::read(&path).map_err(|e| UiError::invalid(format!("cannot read {path}: {e}")))?;
    attributes_import_preview(state, content, config)
}

/// Writes the same plan the preview showed. The preview is rebuilt here rather than sent back
/// by the frontend: what is written is decided by the file and the database, never by the UI.
#[tauri::command]
pub fn attributes_import_commit(
    app: AppHandle,
    state: State<AppState>,
    content: Vec<u8>,
    config: Option<AttributeCsvConfig>,
) -> UiResult<AttributeImportResult> {
    let result = {
        let store = state.store()?;
        let parsed = parse_csv(&content, &ParseConfig::default())?;
        let config = config.unwrap_or_else(|| detect_attribute_config(&parsed));
        let preview = build_attribute_preview(
            &parsed,
            &config,
            &store.list_securities()?,
            &store.list_attribute_defs()?,
        );
        commit_attributes(&store, &preview)?
    };

    emit_changed(&app, "securities")?;
    Ok(result)
}

#[tauri::command]
pub fn attributes_import_commit_path(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    config: Option<AttributeCsvConfig>,
) -> UiResult<AttributeImportResult> {
    let content = std::fs::read(&path).map_err(|e| UiError::invalid(format!("cannot read {path}: {e}")))?;
    attributes_import_commit(app, state, content, config)
}

/// Writes the export where the user points.
#[tauri::command]
pub fn attributes_export_save(state: State<AppState>, path: String) -> UiResult<()> {
    let csv = attributes_export_csv(state)?;
    // Excel reads a comma-separated file as UTF-8 only when it opens with a BOM.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

/// Every instrument and every attribute as one CSV — the file this same import reads back.
#[tauri::command]
pub fn attributes_export_csv(state: State<AppState>) -> UiResult<String> {
    let store = state.store()?;
    Ok(attributes_to_csv(
        &store.list_securities()?,
        &store.list_attribute_defs()?,
        &store.security_attributes()?,
    ))
}
