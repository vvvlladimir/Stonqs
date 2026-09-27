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

/// 1: sources are chosen, never defaulted (ADR-0076).
pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Which build's reading of this file applies; `0` is anything written before it existed.
    #[serde(default)]
    pub version: u32,
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
    /// Where a new chat starts; also the last provider switched to in a footer (ADR-0069).
    #[serde(default = "ai_provider_default")]
    pub ai_provider: String,
    /// Last model per provider, read through `ai::models::remembered`.
    #[serde(default)]
    pub ai_models: std::collections::BTreeMap<String, String>,
    /// Ids the user added per provider, offered after the shortlist.
    #[serde(default)]
    pub ai_extra_models: std::collections::BTreeMap<String, Vec<String>>,
    /// The thinking effort last picked in a chat; where the next chat starts.
    #[serde(default)]
    pub ai_effort: sq_core::model::AiEffort,
    /// Whether the provider may search the web on the user's behalf. A question typed here
    /// leaves the machine when this is on, so it is a setting rather than a default.
    #[serde(default = "ai_web_search_default")]
    pub ai_web_search: bool,
    /// Off by default: it costs tokens and some accounts are refused it outright.
    #[serde(default)]
    pub ai_reasoning: bool,
    /// Offered as a provider only once configured; its key lives in the vault.
    #[serde(default)]
    pub ai_custom: crate::ai::catalog::CustomProvider,
    /// Market-data sources the user switched away from their default (`source id -> on`).
    /// Host state: the host decides what leaves the machine. Own command, `market_source_switch`.
    #[serde(default)]
    pub market_sources: std::collections::BTreeMap<String, bool>,
    /// Until set, every source is off (ADR-0076). Set by `market_sources_confirm`; `settings_save` never clears it.
    #[serde(default)]
    pub sources_configured: bool,
    /// Quote sources the user described (ADR-0054); their keys sit in the vault, not here.
    #[serde(default)]
    pub market_custom: Vec<sq_core::market::CustomSource>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            version: SETTINGS_VERSION,
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
            sources_configured: false,
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

/// A file that exists is migrated, never reset; no file means nothing is switched on.
pub fn load(db_path: &Path) -> AppSettings {
    match std::fs::read(path_for(db_path))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AppSettings>(&bytes).ok())
    {
        Some(mut stored) => {
            migrate(&mut stored);
            stored
        }
        None => AppSettings::default(),
    }
}

/// Sources that were on before ADR-0076 without anybody saying so. The list describes a past
/// state, so it is written out rather than derived from a catalogue that no longer holds it.
const ON_BEFORE_THE_SOURCES_WERE_CHOSEN: &[&str] = &[
    sq_core::market::YahooProvider::ID,
    sq_core::market::KrakenProvider::ID,
];

/// Version 0: the old defaults' silent state is written down, so an update does not stop anyone's quotes.
fn migrate(settings: &mut AppSettings) {
    if settings.version >= SETTINGS_VERSION {
        return;
    }
    for source in sq_core::sources::CATALOG {
        let was_on = source.on_by_default || ON_BEFORE_THE_SOURCES_WERE_CHOSEN.contains(&source.id);
        settings
            .market_sources
            .entry(source.id.to_string())
            .or_insert(was_on);
    }
    settings.sources_configured = true;
    settings.version = SETTINGS_VERSION;
}

pub fn store(db_path: &Path, settings: &AppSettings) -> UiResult<()> {
    let json = serde_json::to_vec_pretty(settings).map_err(|e| UiError::internal(e.to_string()))?;
    std::fs::write(path_for(db_path), json).map_err(|e| UiError::internal(e.to_string()))
}

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> UiResult<AppSettings> {
    Ok(state.settings()?.clone())
}

/// Leaves scope, periods, UI state and the footer's picks alone: each has its own command.
#[tauri::command]
pub fn settings_save(state: State<AppState>, settings: AppSettings) -> UiResult<AppSettings> {
    let scope = state.scope()?.clone();
    // One guard for the whole read: the settings Mutex is not reentrant, and guards taken inside
    // a struct literal all live until the end of that statement, so locking twice deadlocks.
    let (
        periods,
        hidden_presets,
        ui,
        custom,
        market_sources,
        sources_configured,
        market_custom,
        ai_models,
        ai_effort,
    ) = {
        let current = state.settings()?;
        (
            current.periods.clone(),
            current.hidden_presets.clone(),
            current.ui.clone(),
            current.ai_custom.clone(),
            current.market_sources.clone(),
            current.sources_configured,
            current.market_custom.clone(),
            current.ai_models.clone(),
            current.ai_effort,
        )
    };
    // The custom provider's model list was fetched from the address that just changed, so the
    // run's cache of it is now about a different server (`commands::ai::providers::models_for`).
    if custom != settings.ai_custom
        && let Ok(mut cached) = state.ai_models.lock()
    {
        cached.remove(crate::ai::catalog::CUSTOM);
    }
    let settings = AppSettings {
        version: SETTINGS_VERSION,
        scope,
        periods,
        hidden_presets,
        ui,
        market_sources,
        sources_configured,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sq_core::market::{KrakenProvider, YahooProvider};
    use sq_core::sources::{self, Setup};

    /// A file from before ADR-0076 keeps fetching what it fetched: its defaults are written down
    /// before the catalogue stops meaning them, and its owner counts as having chosen.
    #[test]
    fn an_older_file_keeps_the_sources_it_had() {
        let stored: AppSettings = serde_json::from_str(
            r#"{"auto_refresh_on_start":true,"refresh_min_interval_hours":6,"last_refresh":null}"#,
        )
        .unwrap();
        assert_eq!(stored.version, 0);

        let mut migrated = stored;
        migrate(&mut migrated);
        assert!(migrated.sources_configured);
        assert_eq!(migrated.version, SETTINGS_VERSION);
        assert!(migrated.market_sources[YahooProvider::ID]);
        assert!(migrated.market_sources[KrakenProvider::ID]);

        let setup = Setup {
            switched: migrated.market_sources.clone().into_iter().collect(),
            ..Setup::default()
        };
        assert_eq!(sources::default_quotes(&setup), Some(YahooProvider::ID));
    }

    /// A switch the owner made themselves is what it was: the migration fills silence, never a
    /// recorded answer.
    #[test]
    fn a_recorded_switch_survives_the_migration() {
        let mut settings = AppSettings {
            version: 0,
            ..AppSettings::default()
        };
        settings.market_sources.insert(YahooProvider::ID.into(), false);
        migrate(&mut settings);
        assert!(!settings.market_sources[YahooProvider::ID]);
    }

    /// A profile nobody has set up starts with the question unanswered, whatever the catalogue
    /// would have defaulted to.
    #[test]
    fn a_new_profile_has_chosen_nothing() {
        let fresh = AppSettings::default();
        assert!(!fresh.sources_configured);
        assert_eq!(fresh.version, SETTINGS_VERSION);
        assert!(fresh.market_sources.is_empty());
    }
}
