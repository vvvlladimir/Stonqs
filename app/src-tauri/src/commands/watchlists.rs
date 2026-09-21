//! Named lists of instruments and what each instrument's own quotes say. See ADR-0035.
//!
//! Not scoped: a watched instrument's price does not depend on the account picker. The figures
//! of a position held in it are the positions screen's, joined by the frontend.

use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::calc::{InstrumentMove, NearestLevel, alert_status, instrument_move, nearest_level};
use sq_core::prelude::*;
use tauri::{AppHandle, State};

#[derive(Debug, Deserialize)]
pub struct WatchlistInput {
    pub id: Option<String>,
    pub name: String,
    pub security_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WatchRow {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    pub kind: SecurityKind,
    #[serde(flatten)]
    pub quote: InstrumentMove,
    /// The trigger level the price is closest to; `None` without a price rule or a status.
    pub nearest_level: Option<NearestLevel>,
}

#[tauri::command]
pub fn watchlists_list(state: State<AppState>) -> UiResult<Vec<Watchlist>> {
    Ok(state.store()?.list_watchlists()?)
}

#[tauri::command]
pub fn watchlist_save(app: AppHandle, state: State<AppState>, input: WatchlistInput) -> UiResult<Watchlist> {
    let list = Watchlist {
        id: input.id.unwrap_or_else(sq_core::model::new_id),
        name: input.name.trim().to_string(),
        security_ids: input.security_ids,
    };
    state.store()?.save_watchlist(&list)?;
    emit_changed(&app, "watchlists")?;
    Ok(list)
}

#[tauri::command]
pub fn watchlist_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_watchlist(&id)?;
    emit_changed(&app, "watchlists")
}

/// One row per instrument on the list, in the list's order, as of `to`.
#[tauri::command]
pub fn watchlist_rows(
    state: State<AppState>,
    id: String,
    from: String,
    to: String,
) -> UiResult<Vec<WatchRow>> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    if from > to {
        return Err(UiError::invalid(format!(
            "the period starts after it ends: {from} > {to}"
        )));
    }
    let store = state.store()?;
    let list = store.get_watchlist(&id)?;
    let securities = store.list_securities()?;
    let alerts = store.list_alerts()?;

    let mut rows = Vec::with_capacity(list.security_ids.len());
    for security in list
        .security_ids
        .iter()
        .filter_map(|id| securities.iter().find(|s| &s.id == id))
    {
        let prices = store.quote_series(&security.id, to)?;
        let events = store.security_events_for(&security.id)?;
        let mut statuses = Vec::new();
        for alert in alerts.iter().filter(|a| a.security_id == security.id) {
            match alert_status(alert, &prices, &*store, to) {
                Ok(status) => statuses.push((alert, status)),
                // A rule waiting for a quote or a rate is simply not the nearest one yet.
                Err(sq_core::Error::MissingMarketData { .. }) => {}
                Err(e) => return Err(e.into()),
            }
        }
        rows.push(WatchRow {
            security_id: security.id.clone(),
            symbol: security.symbol.clone(),
            name: security.name.clone(),
            kind: security.kind,
            quote: instrument_move(&prices, &events, from, to),
            nearest_level: nearest_level(statuses.iter().map(|(alert, status)| (*alert, status))),
        });
    }
    Ok(rows)
}
