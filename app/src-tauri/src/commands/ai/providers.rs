//! Which provider and model a chat or a tile lands on (ADR-0069).

use crate::ai::{catalog, models};
use crate::error::UiResult;
use crate::settings::AppSettings;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

/// One provider as the picker needs it. `connected` is the vault's yes/no; the key never crosses IPC.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderOption {
    pub id: String,
    pub connected: bool,
    /// Empty for a built-in (a brand name the frontend knows); the user's words for the custom one.
    pub label: String,
}

/// The built-in providers, then the user's own once configured.
#[tauri::command]
pub fn ai_providers_list(state: State<AppState>) -> UiResult<Vec<ProviderOption>> {
    let custom = state.settings()?.ai_custom.clone();
    let mut options: Vec<ProviderOption> = catalog::PROVIDERS
        .iter()
        .map(|provider| ProviderOption {
            id: provider.id.to_string(),
            // An unreadable vault is not a key.
            connected: state.key_exists(provider.id).unwrap_or(false),
            label: String::new(),
        })
        .collect();
    if custom.is_set() {
        options.push(ProviderOption {
            // Its key is optional, so being configured is what makes it usable.
            id: catalog::CUSTOM.to_string(),
            connected: true,
            label: custom.label.trim().to_string(),
        });
    }
    Ok(options)
}

/// What one provider offers; absent `provider` means the one a new chat would start on.
#[tauri::command]
pub fn ai_models_list(state: State<AppState>, provider: Option<String>) -> UiResult<Vec<String>> {
    let provider = match provider {
        Some(id) => id,
        None => state.settings()?.ai_provider.clone(),
    };
    models_for(&state, &provider)
}

/// The provider's shortlist (cached per run), then the ids the user added. The one source of
/// "what is on offer", so the picker and a new chat's model never disagree.
pub(super) fn models_for(state: &AppState, provider: &str) -> UiResult<Vec<String>> {
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
                // An unreadable catalogue still leaves the user's own ids.
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

/// The model last picked at this provider as today's list reads it, else its smallest tier; the
/// compiled-in default only when no list can be read.
pub(super) fn default_model(state: &AppState, provider: &str) -> String {
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
            // Nothing to check it against: the user's pick beats a compiled-in guess.
            None => return chosen,
        }
    }
    listed
        .and_then(|listed| models::smallest(provider, &listed))
        .or_else(|| catalog::default_model(provider).map(str::to_string))
        // The custom provider's model is what the user typed.
        .or_else(|| {
            state
                .settings()
                .ok()
                .map(|settings| settings.ai_custom.model.trim().to_string())
        })
        .unwrap_or_default()
}

/// The first usable provider in picker order; the custom one last.
pub(super) fn connected_provider(state: &AppState, settings: &AppSettings) -> Option<String> {
    catalog::PROVIDERS
        .iter()
        .map(|provider| provider.id.to_string())
        .chain(settings.ai_custom.is_set().then(|| catalog::CUSTOM.to_string()))
        .find(|id| usable(state, settings, id))
}

/// A key for a built-in provider; an address and a model for the custom one.
pub(super) fn usable(state: &AppState, settings: &AppSettings, provider: &str) -> bool {
    if provider == catalog::CUSTOM {
        return settings.ai_custom.is_set();
    }
    state.key_exists(provider).unwrap_or(false)
}
