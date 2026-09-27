//! One chat turn, run on its own thread with its own `Store`.

use super::stream::{AiStreamEvent, PanelGate};
use crate::ai::{catalog, session};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

/// Starts the turn and returns at once; replies arrive through `on_event`. The lock is never held
/// across the network call (`ui-boundary.md`).
#[tauri::command]
pub fn ai_send(
    app: AppHandle,
    state: State<AppState>,
    chat_id: String,
    text: String,
    screen: Option<String>,
    // The date the screens are set to, when it is not today.
    as_of: Option<String>,
    on_event: Channel<AiStreamEvent>,
) -> UiResult<()> {
    let settings = state.settings()?.clone();
    if !settings.ai_enabled {
        return Err(UiError::invalid("the AI panel is turned off"));
    }
    // The chat's provider, resolved here so "no key" is this command's error, not a stream's.
    let chat_provider = state.store()?.ai_chat_get(&chat_id)?.provider;
    let key = state.key_for_call(&chat_provider)?;

    // Snapshotted: moving the picker mid-turn does not change the question asked.
    let scope = {
        let store = state.store()?;
        state.scope_selection(&store)?
    };
    let today = chrono::Local::now().date_naive();
    let as_of = as_of.as_deref().map(crate::commands::parse_date).transpose()?;
    let quotes_source = sq_core::sources::default_quotes(&state.market_setup());
    let access = state.db_access()?;
    // Per message, so a plugin installed mid-chat answers from the next one (ADR-0085).
    let plugin_tools = state.plugins.tools()?;

    state.ai_cancel.store(false, Ordering::Relaxed);
    state.ai_consent.clear();

    std::thread::spawn(move || {
        let app_state = app.state::<AppState>();
        let cancelled = || app_state.ai_cancel.load(Ordering::Relaxed);

        // How a write tool reaches the screens: the usual `data:changed`, plus what a command
        // doing the same write would also do.
        let changed = |scope: &'static str| {
            let _ = emit_changed(&app, scope);
            if matches!(scope, "transactions" | "securities") {
                crate::jobs::fetch_missing(&app, &app_state);
            }
            // The host keeps the portfolio's accounts in memory.
            if scope == "accounts" {
                let _ = app_state.reload_portfolio();
            }
        };

        let _in_use = app_state.db_in_use();
        let turn = access.open().map_err(UiError::from).and_then(|store| {
            let gate = PanelGate {
                pending: &app_state.ai_consent,
                channel: &on_event,
                cancelled: &cancelled,
            };
            let session = session::Session {
                store: &store,
                scope: &scope,
                quotes_source,
                today,
                as_of,
                screen,
                chat_id: &chat_id,
                web_search: settings.ai_web_search,
                reasoning_summary: settings.ai_reasoning,
                gate: &gate,
                cancelled: &cancelled,
                changed: &changed,
                plugin_tools: &plugin_tools,
            };
            let provider = catalog::build(&chat_provider, key, &settings.ai_custom)?;
            session::send(&session, provider.as_ref(), text, &mut |event| {
                let _ = on_event.send(event.into());
            })
            .map_err(UiError::from)
        });

        if let Err(error) = turn {
            let _ = on_event.send(AiStreamEvent::Error { error });
        }
        app_state.ai_consent.clear();
        // The user's message was written either way.
        let _ = emit_changed(&app, "ai_chats");
    });

    Ok(())
}

/// Stops the running turn; what already arrived stays in the chat.
#[tauri::command]
pub fn ai_cancel(state: State<AppState>) -> UiResult<()> {
    state.ai_cancel.store(true, Ordering::Relaxed);
    state.ai_consent.clear();
    Ok(())
}
