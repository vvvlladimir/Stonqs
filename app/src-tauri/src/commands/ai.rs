//! AI panel commands: parse arguments, call the keychain or `ai::*`, map the error. No command
//! here returns a saved key — see `ai::keys`.

use crate::ai::catalog;
use crate::ai::consent::{ConsentGate, Decision};
use crate::ai::store::ChatTurn;
use crate::ai::tools::Params;
use crate::ai::tools::ToolContext;
use crate::ai::{AiEvent, brief, export, models, session, store};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::scope::DataScope;
use crate::state::AppState;
use serde::Serialize;
use sq_core::model::{AiChat, AiEffort, AiToolMode, AiUsage, AiUsageTotal};
use std::sync::atomic::Ordering;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

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

/// A new chat starts on the provider the settings name — unless no key is saved for that one,
/// in which case it starts on a provider that has one. The alternative is a chat that cannot
/// answer until the user finds the picker, and the picker is not where they are looking.
///
/// The model and the effort are the ones last picked in a chat (ADR-0069), the model read against
/// what the provider offers today so a retired id is never replayed; nothing picked yet starts on
/// the smallest tier. The tool mode is deliberately not carried: a chat opened permissively must
/// not make the next one permissive (ADR-0037).
#[tauri::command]
pub fn ai_chat_create(app: AppHandle, state: State<AppState>, title: String) -> UiResult<AiChat> {
    let settings = state.settings()?.clone();
    let starts_on = match usable(&state, &settings, &settings.ai_provider) {
        true => settings.ai_provider.clone(),
        // Nothing connected anywhere: the chat is written on the settings' provider anyway, and
        // the first send says what is missing. Refusing here would be a second way to say it.
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

/// One chat as Markdown, so a conversation can be kept after the history is deleted. Text the
/// user and the model wrote, plus every reading in between — a saved file, not IPC.
#[tauri::command]
pub fn ai_chat_export(state: State<AppState>, id: String) -> UiResult<String> {
    let store = state.store()?;
    let chat = store.ai_chat_get(&id)?;
    Ok(export::markdown(&chat, &store::turns(&store, &id)?))
}

#[tauri::command]
pub fn ai_chat_export_save(state: State<AppState>, id: String, path: String) -> UiResult<()> {
    let text = ai_chat_export(state, id)?;
    // No BOM here, unlike the CSV exports: that one is Excel's price of admission, and every
    // markdown reader takes UTF-8 as it comes.
    std::fs::write(&path, text).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

/// What the user has already allowed for the rest of this chat, so the panel can show it and
/// the list survives a reopened window.
#[tauri::command]
pub fn ai_grants_list(state: State<AppState>, chat_id: String) -> UiResult<Vec<String>> {
    Ok(state.store()?.ai_grants_list(&chat_id)?)
}

/// Answers one consent card. Takes an id and a decision and **nothing else** — the tool and its
/// arguments are looked up in host state, so what was approved and what runs cannot drift apart.
/// See `ai/consent.rs`.
#[tauri::command]
pub fn ai_tool_decide(state: State<AppState>, request_id: String, decision: String) -> UiResult<()> {
    let decision = Decision::from_wire(&decision).ok_or_else(|| UiError::invalid("unknown decision"))?;
    state.ai_consent.decide(&request_id, decision);
    Ok(())
}

/// Switches how this chat treats tool calls. The same switch "allow all" makes on a card, so
/// the toggle and the button cannot mean two different things.
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

/// How hard this chat asks the model to think, switched from inside the chat — and remembered
/// as where the next chat starts.
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

/// Which model answers this chat from here on. The history replays into any adapter, so a chat
/// is never tied to the model it was started with. Remembered per provider for the next chat.
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

/// Which provider answers this chat from here on. The model moves with it: a model id belongs
/// to the catalogue it came from, so the chat lands on the model last picked there (or its
/// smallest tier). The provider becomes where the next chat starts.
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

/// One provider as the picker needs it: what it is, what a chat lands on there, and whether a
/// key is saved for it. `connected` is the keychain's answer and nothing more — the key itself
/// never crosses IPC (`ai::keys`).
#[derive(Debug, Clone, Serialize)]
pub struct ProviderOption {
    pub id: String,
    pub connected: bool,
    /// What to call it. Empty for a built-in — those are brand names the frontend already knows
    /// — and the user's own words for the custom one, which is why it crosses IPC at all.
    pub label: String,
}

/// The providers this build can talk to, in the order the pickers offer them: the built-in ones,
/// then the user's own once it is configured. Each says whether a key is saved for it, and
/// nothing else about that key.
#[tauri::command]
pub fn ai_providers_list(state: State<AppState>) -> UiResult<Vec<ProviderOption>> {
    let custom = state.settings()?.ai_custom.clone();
    let mut options: Vec<ProviderOption> = catalog::PROVIDERS
        .iter()
        .map(|provider| ProviderOption {
            id: provider.id.to_string(),
            // A keychain that cannot be read is not a key: the provider is simply not offered.
            connected: state.key_exists(provider.id).unwrap_or(false),
            label: String::new(),
        })
        .collect();
    if custom.is_set() {
        options.push(ProviderOption {
            // Configured *is* connected here: a key is optional for a server that authenticates
            // nothing, so what makes this one usable is having an address and a model.
            id: catalog::CUSTOM.to_string(),
            connected: true,
            label: custom.label.trim().to_string(),
        });
    }
    Ok(options)
}

/// What one provider says it offers, ranked so the usable ones come first and nothing is hidden
/// — a model matching no rule is still listed, just not at the top. Absent `provider` means the
/// one a new chat would start on.
#[tauri::command]
pub fn ai_models_list(state: State<AppState>, provider: Option<String>) -> UiResult<Vec<String>> {
    let provider = match provider {
        Some(id) => id,
        None => state.settings()?.ai_provider.clone(),
    };
    models_for(&state, &provider)
}

/// The provider's shortlist, asked for once per run, then the ids the user added for it. Every
/// caller goes through here, so the picker and the model a new chat lands on can never disagree
/// about what is on offer. The added ids are joined after the cache, so an edit in Settings
/// shows at once.
fn models_for(state: &AppState, provider: &str) -> UiResult<Vec<String>> {
    let (custom, extra) = {
        let settings = state.settings()?;
        let extra = settings
            .ai_extra_models
            .get(provider)
            .cloned()
            .unwrap_or_default();
        (settings.ai_custom.clone(), extra)
    };
    let cached = state.ai_models.lock().ok().and_then(|c| c.get(provider).cloned());
    let mut listed = match cached {
        Some(listed) => listed,
        None => {
            let listed = state
                .key_for_call(provider)
                .and_then(|key| Ok(models::list(provider, &key, &custom)?));
            match listed {
                Ok(listed) => {
                    if let Ok(mut cache) = state.ai_models.lock() {
                        cache.insert(provider.to_string(), listed.clone());
                    }
                    listed
                }
                // A catalogue that cannot be read still leaves the user's own ids to pick from.
                Err(_) if !extra.is_empty() => Vec::new(),
                Err(error) => return Err(error),
            }
        }
    };
    for id in extra.iter().map(|id| id.trim()).filter(|id| !id.is_empty()) {
        if !listed.iter().any(|listed| listed == id) {
            listed.push(id.to_string());
        }
    }
    Ok(listed)
}

/// What a chat lands on at this provider: the model last picked there, as the list stands today,
/// else the smallest tier on offer. The compiled-in default answers only when the catalogue
/// cannot be read — an id written into this build is out of date the day the provider retires it.
fn default_model(state: &AppState, provider: &str) -> String {
    let chosen = state
        .settings()
        .ok()
        .and_then(|settings| settings.ai_models.get(provider).cloned());
    let listed = models_for(state, provider).ok();
    if let Some(chosen) = chosen {
        match &listed {
            Some(listed) => {
                if let Some(model) = models::remembered(provider, &chosen, listed) {
                    return model;
                }
            }
            // No catalogue to check it against: the user's own pick beats a compiled-in guess.
            None => return chosen,
        }
    }
    listed
        .and_then(|listed| models::smallest(provider, &listed))
        .or_else(|| catalog::default_model(provider).map(str::to_string))
        // The custom provider has no compiled-in default — the model *is* what the user typed.
        .or_else(|| {
            state
                .settings()
                .ok()
                .map(|settings| settings.ai_custom.model.trim().to_string())
        })
        .unwrap_or_default()
}

/// The first provider with a key, in the order the picker offers them. The custom one is last:
/// it only exists once it is configured, and a built-in with a key is the likelier intention.
fn connected_provider(state: &AppState, settings: &crate::settings::AppSettings) -> Option<String> {
    catalog::PROVIDERS
        .iter()
        .map(|provider| provider.id.to_string())
        .chain(settings.ai_custom.is_set().then(|| catalog::CUSTOM.to_string()))
        .find(|id| usable(state, settings, id))
}

/// Whether a turn on this provider would get as far as the network: a key for a built-in one,
/// an address and a model for the custom one, whose key is optional.
fn usable(state: &AppState, settings: &crate::settings::AppSettings, provider: &str) -> bool {
    if provider == catalog::CUSTOM {
        return settings.ai_custom.is_set();
    }
    state.key_exists(provider).unwrap_or(false)
}

/// Stops the running turn. Whatever text already arrived stays in the chat: the user watched it
/// arrive and it was paid for either way.
#[tauri::command]
pub fn ai_cancel(state: State<AppState>) -> UiResult<()> {
    state.ai_cancel.store(true, Ordering::Relaxed);
    state.ai_consent.clear();
    Ok(())
}

/// What travels over the `ai_send` channel. `ai::AiEvent` has no failure variant on purpose: a
/// failed turn arrives here as a `UiError`, a code the frontend has a sentence for, exactly like
/// a failed command — the host never ships the sentence itself (`.claude/rules/ui-boundary.md`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiStreamEvent {
    Text {
        text: String,
    },
    /// The turn is blocked on this card until `ai_tool_decide` answers `request_id`.
    ToolRequested {
        request_id: String,
        tool: String,
        params: Params,
        write: bool,
        /// The model's one line on why. Beside the values on the card, never instead of them.
        reason: String,
    },
    ToolRunning {
        tool: String,
        /// Why the model wanted it — its own words, shown even when the call ran without asking.
        reason: String,
    },
    ToolFinished {
        tool: String,
        content: String,
    },
    Searching {
        query: String,
    },
    /// A chunk of the model's summary of its own reasoning, while it is still thinking.
    Reasoning {
        text: String,
    },
    /// What this turn has cost so far. A running total, so the panel assigns it rather than
    /// adding it up — see `ai::AiEvent::Usage`.
    Usage {
        usage: AiUsage,
    },
    Done,
    Error {
        error: UiError,
    },
}

impl From<AiEvent> for AiStreamEvent {
    fn from(event: AiEvent) -> Self {
        match event {
            AiEvent::Text { text } => AiStreamEvent::Text { text },
            AiEvent::ToolRequested {
                request_id,
                tool,
                params,
                write,
                reason,
            } => AiStreamEvent::ToolRequested {
                request_id,
                tool,
                params,
                write,
                reason,
            },
            AiEvent::ToolRunning { tool, reason } => AiStreamEvent::ToolRunning { tool, reason },
            AiEvent::ToolFinished { tool, content } => AiStreamEvent::ToolFinished { tool, content },
            AiEvent::Searching { query } => AiStreamEvent::Searching { query },
            AiEvent::Reasoning { text } => AiStreamEvent::Reasoning { text },
            AiEvent::Usage { usage } => AiStreamEvent::Usage { usage },
            AiEvent::Done => AiStreamEvent::Done,
        }
    }
}

/// The gate as the panel sees it: draw a card, block, and let `ai_tool_decide` answer it.
struct PanelGate<'a> {
    pending: &'a crate::ai::consent::Pending,
    channel: &'a Channel<AiStreamEvent>,
    cancelled: &'a dyn Fn() -> bool,
}

impl ConsentGate for PanelGate<'_> {
    fn ask(&self, request_id: &str, tool: &str, params: &Params, write: bool, reason: &str) -> Decision {
        let receiver = self.pending.open(request_id);
        let sent = self.channel.send(AiStreamEvent::ToolRequested {
            request_id: request_id.to_string(),
            tool: tool.to_string(),
            params: params.clone(),
            write,
            reason: reason.to_string(),
        });
        if sent.is_err() {
            // Nobody is listening — the window went away mid-turn. Refuse rather than block on a
            // card that will never be drawn.
            self.pending.forget(request_id);
            return Decision::Deny;
        }
        crate::ai::consent::wait(self.pending, request_id, receiver, self.cancelled)
    }
}

/// Fires the turn on its own thread and returns immediately — replies arrive through
/// `on_event`, the same shape `jobs::market_refresh` uses for a long-running job. A network
/// call never runs with `Mutex<Store>` held (`ui-boundary.md`), so the thread opens its own.
#[tauri::command]
pub fn ai_send(
    app: AppHandle,
    state: State<AppState>,
    chat_id: String,
    text: String,
    screen: Option<String>,
    // The date the screens are set to, when the user moved that lens off today.
    as_of: Option<String>,
    on_event: Channel<AiStreamEvent>,
) -> UiResult<()> {
    let settings = state.settings()?.clone();
    if !settings.ai_enabled {
        return Err(UiError::invalid("the AI panel is turned off"));
    }
    // The provider is the chat's, not the app's: two chats open side by side may be answered by
    // different ones. Read here, on the caller's thread, so "no key saved" is the command's own
    // error and the panel can refuse before it starts drawing a reply that will never come.
    let chat_provider = state.store()?.ai_chat_get(&chat_id)?.provider;
    let key = state.key_for_call(&chat_provider)?;

    // The lens is taken once, now, and moved into the thread: a tool answers the question the
    // user asked it from, even if they move the scope picker while the model is thinking.
    let scope = {
        let store = state.store()?;
        state.scope_selection(&store)?
    };
    let today = chrono::Local::now().date_naive();
    let as_of = as_of.as_deref().map(crate::commands::parse_date).transpose()?;
    // Read here for the same reason the lens is: the switches are the host's settings, and the
    // turn runs on a thread that holds no lock on them.
    let quotes_source = sq_core::sources::default_quotes(&state.market_setup());
    let access = state.db_access()?;

    state.ai_cancel.store(false, Ordering::Relaxed);
    state.ai_consent.clear();

    std::thread::spawn(move || {
        let app_state = app.state::<AppState>();
        let cancelled = || app_state.ai_cancel.load(Ordering::Relaxed);

        // What a write tool reaches the screens through. A tool body knows nothing about the
        // host, so the host hands it this: the same `data:changed` every command emits, plus the
        // quote fetch a new instrument or a new trade owes (`.claude/rules/money-and-fx.md`).
        let changed = |scope: &'static str| {
            let _ = emit_changed(&app, scope);
            if matches!(scope, "transactions" | "securities") {
                crate::jobs::fetch_missing(&app, &app_state);
            }
            // An account opened or deleted mid-turn changes which accounts the portfolio holds,
            // and the host keeps that list in memory — without this the next command would read
            // the portfolio as it was before the tool ran.
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
        // Either way the chat's own row moved: the user's message was written before the call.
        let _ = emit_changed(&app, "ai_chats");
    });

    Ok(())
}

/// The dashboard brief: a fixed set of readings, one model call, no tools and no history.
///
/// It takes a resolved window like every other reporting command — the tile already knows the
/// dates it was drawn for, and resolving the period a second time here could disagree with them.
/// Consent was given when the widget was placed: the readings are `brief::READINGS`, named in the
/// host, so nothing the model says can widen them (ADR-0039).
// A command's arguments are its IPC payload, one per field the frontend sends.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn ai_brief(
    app: AppHandle,
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
    // `instructions` is the tile's own, as the user typed it in the widget's settings — their
    // words, crossing IPC the way a chat's first message does, with no sentence authored here.
    // `language` is the interface's locale tag: a tile has no question to read the language off,
    // and it answers in the language of the app it sits in, not of the data.
    instructions: Option<String>,
    language: String,
    // The tile's own choice, from its settings. Absent, it follows where a new chat begins, model
    // included.
    provider: Option<String>,
    model: Option<String>,
    // The tile's answer length in output tokens; absent, the adapters' own ceiling.
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
    // Resolved before the thread starts, for the reason a chat resolves it at creation: the tile
    // asks the provider what it serves today rather than replaying an id from a settings file.
    let model = chosen(model).unwrap_or_else(|| default_model(&state, &provider_id));
    let range = crate::commands::performance::date_range(&from, &to)?;

    let scope = {
        let store = state.store()?;
        state.scope_selection_in(&store, source.as_ref())?
    };
    let today = chrono::Local::now().date_naive();
    let access = state.db_access()?;

    // The readings run on this thread's own `Store`, like every other job that ends in a network
    // call: `Mutex<Store>` is never held across one (`.claude/rules/ui-boundary.md`).
    std::thread::spawn(move || {
        // The streamed text is what the tile keeps; the return value only matters to the tests.
        let app_state = app.state::<AppState>();
        let _in_use = app_state.db_in_use();
        let written = access.open().map_err(UiError::from).and_then(|store| {
            let context = ToolContext {
                store: &store,
                scope: &scope,
                // The brief neither creates an instrument nor re-points one.
                quotes_source: None,
                today,
                // The brief only reads: its readings are a fixed list of read tools (ADR-0039).
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

            // A brief has no chat behind it, so it is recorded against the model alone — the
            // spending happened and belongs in the settings total either way (ADR-0041).
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

/// Everything the assistant has spent, by model. Counted by the provider and only ever read
/// here — the app never prices it, because a price list in the binary is a price list that is
/// wrong by the next release (ADR-0041).
#[tauri::command]
pub fn ai_usage_totals(state: State<AppState>) -> UiResult<Vec<AiUsageTotal>> {
    Ok(state.store()?.ai_usage_totals()?)
}
