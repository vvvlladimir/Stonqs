use crate::error::{UiError, UiResult};
#[cfg(debug_assertions)]
use crate::state::AppState;
#[cfg(debug_assertions)]
use tauri::State;

/// What the alert simulator does to one rule.
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevAlertStep {
    /// Writes today's close one percent past the level, on the side the price is not on; a date
    /// rule is moved to today.
    Cross,
    /// Deletes the simulated quotes of the rule's instrument, so the price returns to the real one.
    Reset,
}

/// Source of simulated quotes, so `Reset` removes exactly those and a refresh overwrites them.
#[cfg(debug_assertions)]
const DEV_SOURCE: &str = "dev";

/// Moves a real quote rather than faking a crossing: the check that runs after it is the same one
/// a refresh runs, so the log, the dot and the notification are what a real move would produce.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn dev_alert_simulate(
    app: tauri::AppHandle,
    state: State<AppState>,
    alert_id: String,
    step: DevAlertStep,
) -> UiResult<usize> {
    use crate::commands::alerts::{check_alerts, today};
    use sq_core::prelude::*;

    let today = today();
    let logged = {
        let store = state.store()?;
        // The simulated close is placed against the side the real quotes leave the rule on.
        check_alerts(&store, today)?;
        let mut alert = store.get_alert(&alert_id)?;
        match (step, alert.kind) {
            (DevAlertStep::Cross, AlertKind::Price) => {
                let close = sq_core::calc::crossing_close(&alert)
                    .ok_or_else(|| UiError::invalid("the alert has no level"))?;
                store.save_quotes(&[Quote {
                    security_id: alert.security_id.clone(),
                    date: today,
                    close,
                    currency: alert.currency.clone().unwrap_or_default(),
                    source: DEV_SOURCE.into(),
                }])?;
            }
            (DevAlertStep::Cross, AlertKind::DateReached) => {
                alert.date = Some(today);
                alert.checked_through = None;
                store.save_alert(&alert)?;
            }
            (DevAlertStep::Reset, _) => {
                store.delete_quotes_from(&alert.security_id, DEV_SOURCE)?;
            }
        }
        check_alerts(&store, today)?
    };
    crate::events::emit_changed(&app, "quotes")?;
    crate::events::emit_changed(&app, "alerts")?;
    Ok(logged)
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn dev_alert_simulate() -> UiResult<usize> {
    Err(UiError::invalid(
        "the alert simulator is only available in a debug build",
    ))
}
