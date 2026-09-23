//! Application settings stored in JSON beside the database.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::calc::PeriodSpec;
use std::path::{Path, PathBuf};
use tauri::State;

/// A period the user added to the axis. The host stores it and hands the spec to the core to
/// resolve; the name is the user's own data and is never translated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPeriod {
    pub id: String,
    pub name: String,
    pub spec: PeriodSpec,
}

/// What the period editor edits: the user's own periods and the shipped presets they hid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodSettings {
    pub periods: Vec<UserPeriod>,
    pub hidden_presets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Refresh quotes and FX rates on startup.
    pub auto_refresh_on_start: bool,
    /// Minimum interval between automatic refreshes.
    pub refresh_min_interval_hours: i64,
    /// Last completed refresh time in RFC 3339.
    pub last_refresh: Option<String>,
    /// Selected data scope, persisted across restarts.
    #[serde(default)]
    pub scope: crate::scope::DataScope,
    /// UI language: "system", or a locale tag the frontend ships. The host only stores it.
    #[serde(default = "system_language")]
    pub language: String,
    /// Periods the user added to the axis. Host state rather than the opaque `ui` blob:
    /// the host has to resolve them through the core before the frontend sees dates.
    #[serde(default)]
    pub periods: Vec<UserPeriod>,
    /// Shipped presets the user removed from the strip, written down rather than deleted.
    #[serde(default)]
    pub hidden_presets: Vec<String>,
    /// Frontend-owned UI state kept opaque to the host.
    #[serde(default)]
    pub ui: serde_json::Value,
    /// Whether the AI panel is offered at all. A provider key can exist while this is off.
    #[serde(default)]
    pub ai_enabled: bool,
    /// Selected provider id (e.g. `"openai"`); keychain account name and command argument.
    /// Also what the chat footer was last switched to: a new chat starts where the last choice
    /// left off (ADR-0069).
    #[serde(default = "ai_provider_default")]
    pub ai_provider: String,
    /// The model last picked in a chat, per provider (`provider -> model id`). Absent, a chat
    /// starts on that provider's smallest tier. Read through `ai::models::remembered`, so an id
    /// the provider has since retired gives way to the newest of its tier.
    #[serde(default)]
    pub ai_models: std::collections::BTreeMap<String, String>,
    /// Model ids the user typed in per provider (`provider -> ids`), offered in the chat's picker
    /// after the provider's own shortlist: a model the shortlist leaves out, or one the
    /// provider's catalogue does not list at all, is still reachable.
    #[serde(default)]
    pub ai_extra_models: std::collections::BTreeMap<String, Vec<String>>,
    /// The thinking effort last picked in a chat; where the next chat starts.
    #[serde(default)]
    pub ai_effort: sq_core::model::AiEffort,
    /// Whether the provider may search the web on the user's behalf. A question typed here
    /// leaves the machine when this is on, so it is a setting rather than a default.
    #[serde(default = "ai_web_search_default")]
    pub ai_web_search: bool,
    /// Whether the panel shows the model's summary of its own reasoning. Off by default, and
    /// deliberately not by the same argument as web search: the summary costs output tokens and
    /// some provider accounts are refused it outright, which would fail the whole request — a
    /// feature nobody asked for must not be able to break a chat.
    #[serde(default)]
    pub ai_reasoning: bool,
    /// The server the user points the app at themselves, when they do. Empty until configured,
    /// and offered as a provider only then (`ai::catalog::CustomProvider::is_set`). Its key is
    /// not here — that lives in the keychain, like every other provider's.
    #[serde(default)]
    pub ai_custom: crate::ai::catalog::CustomProvider,
    /// Market-data sources the user switched away from their default (`source id -> on`).
    /// Host state: the host decides what leaves the machine. Own command, `market_source_switch`.
    #[serde(default)]
    pub market_sources: std::collections::BTreeMap<String, bool>,
    /// Quote sources the user described (ADR-0054); their keys sit in the vault, not here.
    #[serde(default)]
    pub market_custom: Vec<sq_core::market::CustomSource>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            auto_refresh_on_start: true,
            refresh_min_interval_hours: 6,
            last_refresh: None,
            scope: crate::scope::DataScope::portfolio(),
            language: system_language(),
            periods: Vec::new(),
            hidden_presets: Vec::new(),
            // `null` means the frontend has not saved a layout yet.
            ui: serde_json::Value::Null,
            ai_enabled: false,
            ai_provider: ai_provider_default(),
            ai_models: Default::default(),
            ai_extra_models: Default::default(),
            ai_effort: Default::default(),
            ai_web_search: ai_web_search_default(),
            ai_reasoning: false,
            ai_custom: crate::ai::catalog::CustomProvider::default(),
            market_sources: Default::default(),
            market_custom: Vec::new(),
        }
    }
}

fn system_language() -> String {
    "system".to_string()
}

fn ai_provider_default() -> String {
    "openai".to_string()
}

/// On by default: an assistant that cannot look anything up is the surprising one, and the
/// panel is off entirely until the user turns it on and saves a key.
fn ai_web_search_default() -> bool {
    true
}

impl AppSettings {
    /// Whether an automatic refresh is due.
    pub fn due(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if !self.auto_refresh_on_start {
            return false;
        }
        let Some(last) = self.last_refresh.as_deref() else {
            return true;
        };
        match chrono::DateTime::parse_from_rfc3339(last) {
            Ok(last) => {
                now - last.with_timezone(&chrono::Utc)
                    >= chrono::Duration::hours(self.refresh_min_interval_hours)
            }
            Err(_) => true,
        }
    }
}

pub fn path_for(db_path: &Path) -> PathBuf {
    db_path.with_file_name("settings.json")
}

/// Load settings; missing or invalid files use defaults.
pub fn load(db_path: &Path) -> AppSettings {
    std::fs::read(path_for(db_path))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn store(db_path: &Path, settings: &AppSettings) -> UiResult<()> {
    let json = serde_json::to_vec_pretty(settings).map_err(|e| UiError::internal(e.to_string()))?;
    std::fs::write(path_for(db_path), json).map_err(|e| UiError::internal(e.to_string()))
}

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> UiResult<AppSettings> {
    Ok(state.settings()?.clone())
}

/// Save settings without overwriting the active scope, the period axis, frontend UI state or the
/// chat footer's last picks: each of those has its own command, and this one must not roll them
/// back.
#[tauri::command]
pub fn settings_save(state: State<AppState>, settings: AppSettings) -> UiResult<AppSettings> {
    let scope = state.scope()?.clone();
    // One guard for the whole read: the settings Mutex is not reentrant, and guards taken inside
    // a struct literal all live until the end of that statement, so locking twice deadlocks.
    let (periods, hidden_presets, ui, custom, market_sources, market_custom, ai_models, ai_effort) = {
        let current = state.settings()?;
        (
            current.periods.clone(),
            current.hidden_presets.clone(),
            current.ui.clone(),
            current.ai_custom.clone(),
            current.market_sources.clone(),
            current.market_custom.clone(),
            current.ai_models.clone(),
            current.ai_effort,
        )
    };
    // The custom provider's model list was fetched from the address that just changed, so the
    // run's cache of it is now about a different server (`commands::ai::models_for`).
    if custom != settings.ai_custom
        && let Ok(mut cached) = state.ai_models.lock()
    {
        cached.remove(crate::ai::catalog::CUSTOM);
    }
    let settings = AppSettings {
        scope,
        periods,
        hidden_presets,
        ui,
        market_sources,
        market_custom,
        ai_models,
        ai_effort,
        ..settings
    };
    store(&state.db_path()?, &settings)?;
    *state.settings()? = settings.clone();
    Ok(settings)
}

/// Save frontend UI state without changing other settings.
#[tauri::command]
pub fn ui_state_save(state: State<AppState>, ui: serde_json::Value) -> UiResult<()> {
    state.settings()?.ui = ui;
    state.persist_settings()
}
