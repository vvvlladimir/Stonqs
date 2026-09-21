use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::prelude::*;
use std::str::FromStr;
use tauri::{AppHandle, State};

/// One action with the instrument named, so the list reads without a second query.
#[derive(Debug, Serialize)]
pub struct CorporateActionRow {
    #[serde(flatten)]
    pub action: CorporateAction,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CorporateActionInput {
    pub id: Option<String>,
    pub security_id: String,
    pub date: String,
    /// Ratio as the user writes it: a 2:1 split is `from = "1"`, `to = "2"`.
    pub ratio_from: String,
    pub ratio_to: String,
    pub note: Option<String>,
}

/// Every action, or those of one instrument. Not scoped: a split is a fact about the
/// instrument, and the account picker does not change whether the shares were doubled.
#[tauri::command]
pub fn corporate_actions_list(
    state: State<AppState>,
    security_id: Option<String>,
) -> UiResult<Vec<CorporateActionRow>> {
    let store = state.store()?;
    let actions = match security_id.as_deref() {
        Some(id) => store.corporate_actions_for_security(id)?,
        None => store.list_corporate_actions()?,
    };
    let securities = store.list_securities()?;

    Ok(actions
        .into_iter()
        .map(|action| {
            let security = securities.iter().find(|s| s.id == action.security_id);
            CorporateActionRow {
                symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                action,
            }
        })
        .collect())
}

#[tauri::command]
pub fn corporate_action_save(
    app: AppHandle,
    state: State<AppState>,
    input: CorporateActionInput,
) -> UiResult<CorporateAction> {
    let date = parse_date(&input.date)?;
    let ratio_from = parse_ratio(&input.ratio_from, "ratio_from")?;
    let ratio_to = parse_ratio(&input.ratio_to, "ratio_to")?;

    let action = {
        let store = state.store()?;
        // Fail before writing rather than leaving an action pointing at nothing.
        store.get_security(&input.security_id)?;
        let base = CorporateAction::split(&input.security_id, date, ratio_from, ratio_to);
        let action = CorporateAction {
            id: input.id.unwrap_or(base.id),
            note: input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
            ..base
        };
        store.save_corporate_action(&action)?;
        action
    };

    // A split moves quantities, so every holding, valuation and series moves with it; the
    // "securities" group already invalidates the lists and every computed number.
    emit_changed(&app, "securities")?;
    Ok(action)
}

#[tauri::command]
pub fn corporate_action_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_corporate_action(&id)?;
    emit_changed(&app, "securities")
}

fn parse_ratio(value: &str, field: &str) -> UiResult<Decimal> {
    let raw = value.trim();
    let ratio =
        Decimal::from_str(raw).map_err(|e| UiError::invalid(format!("invalid {field} {raw:?}: {e}")))?;
    if ratio <= Decimal::ZERO {
        return Err(UiError::invalid(format!("{field} must be greater than zero")));
    }
    Ok(ratio)
}
