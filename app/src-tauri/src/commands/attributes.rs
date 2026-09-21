use crate::commands::portfolio::require_text;
use crate::error::UiResult;
use crate::events::emit_changed;
use crate::state::AppState;
use serde::Deserialize;
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
