//! The dashboard's summary tile: fixed readings, one model call, no tools (ADR-0039).

use super::providers::default_model;
use super::stream::AiStreamEvent;
use crate::ai::tools::ToolContext;
use crate::ai::{AiEvent, brief, catalog};
use crate::error::{UiError, UiResult};
use crate::scope::DataScope;
use crate::state::AppState;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

/// Takes the tile's resolved window, like every reporting command.
// A command's arguments are its IPC payload, one per field the frontend sends.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn ai_brief(
    app: AppHandle,
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
    // The user's own words from the tile's settings (ADR-0040).
    instructions: Option<String>,
    // The interface's locale tag: the tile answers in the app's language.
    language: String,
    // Absent: where a new chat begins, model included.
    provider: Option<String>,
    model: Option<String>,
    // Output tokens; absent, the adapter's own ceiling.
    max_tokens: Option<u32>,
    on_event: Channel<AiStreamEvent>,
) -> UiResult<()> {
    let settings = state.settings()?.clone();
    if !settings.ai_enabled {
        return Err(UiError::invalid("the AI panel is turned off"));
    }
    let chosen = |value: Option<String>| value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let provider_id = chosen(provider).unwrap_or_else(|| settings.ai_provider.clone());
    let key = state.key_for_call(&provider_id)?;
    let model = chosen(model).unwrap_or_else(|| default_model(&state, &provider_id));
    let range = crate::commands::performance::date_range(&from, &to)?;

    let scope = {
        let store = state.store()?;
        state.scope_selection_in(&store, source.as_ref())?
    };
    let today = chrono::Local::now().date_naive();
    let access = state.db_access()?;

    std::thread::spawn(move || {
        let app_state = app.state::<AppState>();
        let _in_use = app_state.db_in_use();
        let written = access.open().map_err(UiError::from).and_then(|store| {
            let context = ToolContext {
                store: &store,
                scope: &scope,
                // Read-only: it neither creates nor re-points an instrument.
                quotes_source: None,
                today,
                changed: &|_| {},
            };
            let provider = catalog::build(&provider_id, key, &settings.ai_custom)?;
            let options = brief::Options {
                instructions,
                language,
                max_tokens: max_tokens.map(|limit| limit.max(brief::MIN_TOKENS)),
            };
            let brief = brief::generate(
                &context,
                range,
                &model,
                &options,
                provider.as_ref(),
                &mut |event: AiEvent| {
                    let _ = on_event.send(event.into());
                },
            )
            .map_err(UiError::from)?;

            // No chat behind it: recorded against the model alone (ADR-0041).
            if !brief.usage.is_zero() {
                let _ = store.ai_usage_record(None, &provider_id, &model, brief.usage);
            }
            Ok(brief)
        });

        match written {
            Ok(_) => {
                let _ = on_event.send(AiStreamEvent::Done);
            }
            Err(error) => {
                let _ = on_event.send(AiStreamEvent::Error { error });
            }
        }
    });

    Ok(())
}
