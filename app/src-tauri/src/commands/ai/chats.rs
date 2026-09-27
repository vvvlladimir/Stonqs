//! Provider keys, chats and what is remembered about them.

use super::providers::{connected_provider, default_model, usable};
use crate::ai::catalog;
use crate::ai::consent::Decision;
use crate::ai::store::ChatTurn;
use crate::ai::{export, store};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use sq_core::model::{AiChat, AiEffort, AiToolMode, AiUsageTotal};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn ai_key_save(state: State<AppState>, provider: String, key: String) -> UiResult<()> {
    state.key_save(&provider, &key)
}

#[tauri::command]
pub fn ai_key_delete(state: State<AppState>, provider: String) -> UiResult<()> {
    state.key_delete(&provider)
}

#[tauri::command]
pub fn ai_key_status(state: State<AppState>, provider: String) -> UiResult<bool> {
    state.key_exists(&provider)
}

#[tauri::command]
pub fn ai_chats_list(state: State<AppState>) -> UiResult<Vec<AiChat>> {
    Ok(state.store()?.ai_chats_list()?)
}

/// Starts on the settings' provider, or on one that has a key if that one does not, with the
/// model and effort last picked (ADR-0069). The tool mode is never carried (ADR-0037).
#[tauri::command]
pub fn ai_chat_create(app: AppHandle, state: State<AppState>, title: String) -> UiResult<AiChat> {
    let settings = state.settings()?.clone();
    let starts_on = match usable(&state, &settings, &settings.ai_provider) {
        true => settings.ai_provider.clone(),
        // Nothing connected anywhere: the first send says what is missing.
        _ => connected_provider(&state, &settings).unwrap_or_else(|| settings.ai_provider.clone()),
    };
    let (provider, model) = (starts_on.clone(), default_model(&state, &starts_on));
    let store = state.store()?;
    let mut chat = store.ai_chat_create(&title, &provider, &model)?;
    if settings.ai_effort != chat.effort {
        store.ai_chat_set_effort(&chat.id, settings.ai_effort)?;
        chat = store.ai_chat_get(&chat.id)?;
    }
    drop(store);
    emit_changed(&app, "ai_chats")?;
    Ok(chat)
}

#[tauri::command]
pub fn ai_chat_rename(app: AppHandle, state: State<AppState>, id: String, title: String) -> UiResult<()> {
    state.store()?.ai_chat_rename(&id, &title)?;
    emit_changed(&app, "ai_chats")
}

#[tauri::command]
pub fn ai_chat_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.ai_chat_delete(&id)?;
    emit_changed(&app, "ai_chats")
}

#[tauri::command]
pub fn ai_messages_list(state: State<AppState>, chat_id: String) -> UiResult<Vec<ChatTurn>> {
    let store = state.store()?;
    Ok(store::turns(&store, &chat_id)?)
}

/// One chat as Markdown, readings included.
#[tauri::command]
pub fn ai_chat_export(state: State<AppState>, id: String) -> UiResult<String> {
    let store = state.store()?;
    let chat = store.ai_chat_get(&id)?;
    Ok(export::markdown(&chat, &store::turns(&store, &id)?))
}

#[tauri::command]
pub fn ai_chat_export_save(state: State<AppState>, id: String, path: String) -> UiResult<()> {
    let text = ai_chat_export(state, id)?;
    // No BOM: that is Excel's price for a CSV, and Markdown readers take plain UTF-8.
    std::fs::write(&path, text).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

/// The tools allowed for the rest of this chat.
#[tauri::command]
pub fn ai_grants_list(state: State<AppState>, chat_id: String) -> UiResult<Vec<String>> {
    Ok(state.store()?.ai_grants_list(&chat_id)?)
}

/// Answers one consent card by id alone: the tool and its arguments are looked up in host state,
/// so what was approved and what runs cannot drift apart.
#[tauri::command]
pub fn ai_tool_decide(state: State<AppState>, request_id: String, decision: String) -> UiResult<()> {
    let decision = Decision::from_wire(&decision).ok_or_else(|| UiError::invalid("unknown decision"))?;
    state.ai_consent.decide(&request_id, decision);
    Ok(())
}

/// The same switch a card's "allow all" makes.
#[tauri::command]
pub fn ai_chat_set_mode(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    mode: AiToolMode,
) -> UiResult<()> {
    state.store()?.ai_chat_set_mode(&id, mode)?;
    emit_changed(&app, "ai_chats")
}

/// Remembered as where the next chat starts.
#[tauri::command]
pub fn ai_chat_set_effort(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    effort: AiEffort,
) -> UiResult<()> {
    state.store()?.ai_chat_set_effort(&id, effort)?;
    state.settings()?.ai_effort = effort;
    state.persist_settings()?;
    emit_changed(&app, "ai_chats")
}

/// Remembered per provider for the next chat.
#[tauri::command]
pub fn ai_chat_set_model(app: AppHandle, state: State<AppState>, id: String, model: String) -> UiResult<()> {
    let provider = {
        let store = state.store()?;
        store.ai_chat_set_model(&id, &model)?;
        store.ai_chat_get(&id)?.provider
    };
    state.settings()?.ai_models.insert(provider, model);
    state.persist_settings()?;
    emit_changed(&app, "ai_chats")
}

/// The model moves with the provider: an id belongs to the catalogue it came from.
#[tauri::command]
pub fn ai_chat_set_provider(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    provider: String,
) -> UiResult<()> {
    let custom = state.settings()?.ai_custom.clone();
    if !catalog::is_known(&provider, &custom) {
        return Err(UiError::invalid("unknown provider"));
    }
    let model = default_model(&state, &provider);
    state.store()?.ai_chat_set_provider(&id, &provider, &model)?;
    state.settings()?.ai_provider = provider;
    state.persist_settings()?;
    emit_changed(&app, "ai_chats")
}

/// Everything spent, by model. Counted, never priced (ADR-0041).
#[tauri::command]
pub fn ai_usage_totals(state: State<AppState>) -> UiResult<Vec<AiUsageTotal>> {
    Ok(state.store()?.ai_usage_totals()?)
}
