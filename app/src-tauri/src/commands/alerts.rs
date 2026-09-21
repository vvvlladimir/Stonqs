//! Price triggers, date rules, their crossing log, and dated events of an instrument. See ADR-0034.
//!
//! Not scoped: a trigger is about an instrument, and the account picker does not change whether
//! its price crossed a level.

use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::{AlertStatus, alert_status, check_alert};
use sq_core::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use tauri::{AppHandle, State};

/// Crossings shown under each rule; the full log is `alert_crossings_list`.
const LOG_PER_ALERT: usize = 5;
const LOG_DEFAULT_LIMIT: usize = 50;

/// Why a trigger's status could not be read. A code, never a sentence (ADR-0023).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertProblem {
    /// The instrument has no quote yet.
    MissingPrice,
    /// The quote is in another currency than the level, and that rate is missing.
    MissingRate,
}

#[derive(Debug, Serialize)]
pub struct AlertRow {
    pub alert: SecurityAlert,
    pub symbol: String,
    pub name: String,
    /// `None` exactly when `problem` says why.
    pub status: Option<AlertStatus>,
    pub problem: Option<AlertProblem>,
    /// The latest crossings of this rule, newest first.
    pub crossings: Vec<AlertCrossing>,
}

/// A log line with its rule and instrument named, so the log reads without a join.
#[derive(Debug, Serialize)]
pub struct CrossingRow {
    #[serde(flatten)]
    pub crossing: AlertCrossing,
    pub kind: AlertKind,
    pub note: Option<String>,
    pub security_id: String,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AlertInput {
    pub id: Option<String>,
    pub security_id: String,
    pub kind: AlertKind,
    /// The level of a price trigger, as typed.
    pub price: Option<String>,
    /// Absent: the currency the instrument's quotes arrive in.
    pub currency: Option<String>,
    /// The day of a date rule.
    pub date: Option<String>,
    /// Which crossings a price trigger logs; absent means both.
    pub direction: Option<AlertDirection>,
    pub note: Option<String>,
}

/// An event with the instrument named, so a list across instruments reads without a join.
#[derive(Debug, Serialize)]
pub struct SecurityEventRow {
    #[serde(flatten)]
    pub event: SecurityEvent,
    pub symbol: String,
    pub name: String,
    /// A reported split that is already a corporate action; always false for other kinds.
    pub recorded: bool,
}

/// Only a note is typed by the user; dividends and splits come from the quote provider.
#[derive(Debug, Deserialize)]
pub struct SecurityEventInput {
    pub id: Option<String>,
    pub security_id: String,
    pub date: String,
    pub note: String,
}

pub(crate) fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn clean(note: Option<String>) -> Option<String> {
    note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty())
}

type Series = BTreeMap<NaiveDate, PricePoint>;
static NO_QUOTES: Series = BTreeMap::new();

/// Each instrument's quotes, read once per call; a date rule reads none.
fn prices_for<'a>(
    store: &Store,
    alert: &SecurityAlert,
    series: &'a mut HashMap<String, Series>,
    today: NaiveDate,
) -> sq_core::Result<&'a Series> {
    if alert.kind == AlertKind::DateReached {
        return Ok(&NO_QUOTES);
    }
    if !series.contains_key(&alert.security_id) {
        let loaded = store.quote_series(&alert.security_id, today)?;
        series.insert(alert.security_id.clone(), loaded);
    }
    Ok(&series[&alert.security_id])
}

/// Runs every rule over the quotes on file and logs what crossed; returns how many crossings.
/// A rule whose quote needs a missing rate waits for it rather than failing the rest.
pub fn check_alerts(store: &Store, today: NaiveDate) -> sq_core::Result<usize> {
    let mut series = HashMap::new();
    let mut logged = 0;
    for alert in store.list_alerts()? {
        let prices = prices_for(store, &alert, &mut series, today)?;
        match check_alert(&alert, prices, store, today) {
            Ok(check) if check.changes(&alert) => {
                store.record_alert_check(&alert.id, check.side, check.checked_through, &check.crossings)?;
                logged += check.crossings.len();
            }
            Ok(_) | Err(sq_core::Error::MissingMarketData { .. }) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(logged)
}

/// Every rule, or those of one instrument, with where the price stands and its latest crossings.
#[tauri::command]
pub fn alerts_list(state: State<AppState>, security_id: Option<String>) -> UiResult<Vec<AlertRow>> {
    let store = state.store()?;
    let alerts = match security_id.as_deref() {
        Some(id) => store.alerts_for_security(id)?,
        None => store.list_alerts()?,
    };
    let today = today();
    let securities = store.list_securities()?;
    let mut series = HashMap::new();
    let mut rows = Vec::with_capacity(alerts.len());
    for alert in alerts {
        let prices = prices_for(&store, &alert, &mut series, today)?;
        let (status, problem) = match alert_status(&alert, prices, &*store, today) {
            Ok(status) => (Some(status), None),
            Err(sq_core::Error::MissingMarketData { kind: "price", .. }) => {
                (None, Some(AlertProblem::MissingPrice))
            }
            Err(sq_core::Error::MissingMarketData { .. }) => (None, Some(AlertProblem::MissingRate)),
            Err(e) => return Err(e.into()),
        };
        let security = securities.iter().find(|s| s.id == alert.security_id);
        rows.push(AlertRow {
            symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
            name: security.map(|s| s.name.clone()).unwrap_or_default(),
            crossings: store.alert_crossings(Some(&alert.id), LOG_PER_ALERT)?,
            alert,
            status,
            problem,
        });
    }
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(rows)
}

#[tauri::command]
pub fn alert_save(app: AppHandle, state: State<AppState>, input: AlertInput) -> UiResult<SecurityAlert> {
    let today = today();
    let alert = {
        let store = state.store()?;
        let security = store.get_security(&input.security_id)?;
        let base = match input.kind {
            AlertKind::DateReached => {
                let date = input
                    .date
                    .as_deref()
                    .ok_or_else(|| UiError::invalid("a date alert needs a date"))?;
                SecurityAlert::date_reached(&security.id, parse_date(date)?, today)
            }
            AlertKind::Price => {
                let raw = input.price.as_deref().unwrap_or_default().trim();
                let level = Decimal::from_str(raw)
                    .map_err(|e| UiError::invalid(format!("invalid level {raw:?}: {e}")))?;
                let currency = match input.currency {
                    Some(currency) => currency,
                    None => store
                        .latest_quote_currency(&security.id)?
                        .unwrap_or_else(|| security.currency.clone()),
                };
                SecurityAlert::price(&security.id, level, &currency, today)
                    .with_direction(input.direction.unwrap_or_default())
            }
        };
        let mut alert = SecurityAlert {
            note: clean(input.note),
            ..base
        };

        if let Some(id) = input.id {
            let existing = store.get_alert(&id)?;
            alert.id = id;
            // A new note keeps the bookmark; a new level starts again from today's price.
            if existing.same_trigger(&alert) {
                alert.created_on = existing.created_on;
                alert.side = existing.side;
                alert.checked_through = existing.checked_through;
            }
        }
        store.save_alert(&alert)?;
        check_alerts(&store, today)?;
        alert
    };
    emit_changed(&app, "alerts")?;
    Ok(alert)
}

#[tauri::command]
pub fn alert_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_alert(&id)?;
    emit_changed(&app, "alerts")
}

/// The crossing log across every rule, newest first.
#[tauri::command]
pub fn alert_crossings_list(state: State<AppState>, limit: Option<usize>) -> UiResult<Vec<CrossingRow>> {
    let store = state.store()?;
    let crossings = store.alert_crossings(None, limit.unwrap_or(LOG_DEFAULT_LIMIT))?;
    crossing_rows(&store, crossings)
}

/// How many crossings the user has not looked at; the navigation shows a dot while any are left.
#[tauri::command]
pub fn alerts_unseen(state: State<AppState>) -> UiResult<usize> {
    Ok(state.store()?.unseen_crossings()?)
}

/// Marks the whole log as looked at. Emits nothing: the log on screen keeps its highlight until
/// it is next read, and only the navigation dot has to go.
#[tauri::command]
pub fn alerts_mark_seen(state: State<AppState>) -> UiResult<usize> {
    Ok(state.store()?.mark_crossings_seen()?)
}

/// Checks every rule, then hands over the crossings nobody was told about, marked as told in the
/// same call. The frontend writes the notification: the host knows no language (ADR-0023), and
/// the startup refresh can finish before any window listens, so the UI asks.
#[tauri::command]
pub fn alerts_take_notifications(app: AppHandle, state: State<AppState>) -> UiResult<Vec<CrossingRow>> {
    let (logged, rows) = {
        let store = state.store()?;
        let logged = check_alerts(&store, today())?;
        let taken = store.take_unnotified_crossings()?;
        (logged, crossing_rows(&store, taken)?)
    };
    if logged > 0 {
        emit_changed(&app, "alerts")?;
    }
    Ok(rows)
}

fn crossing_rows(store: &Store, crossings: Vec<AlertCrossing>) -> UiResult<Vec<CrossingRow>> {
    let alerts: HashMap<String, SecurityAlert> = store
        .list_alerts()?
        .into_iter()
        .map(|a| (a.id.clone(), a))
        .collect();
    let securities = store.list_securities()?;
    Ok(crossings
        .into_iter()
        .filter_map(|crossing| {
            let alert = alerts.get(&crossing.alert_id)?;
            let security = securities.iter().find(|s| s.id == alert.security_id);
            Some(CrossingRow {
                kind: alert.kind,
                note: alert.note.clone(),
                security_id: alert.security_id.clone(),
                symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                crossing,
            })
        })
        .collect())
}

/// Events of one instrument, or of every instrument; newest first.
#[tauri::command]
pub fn security_events_list(
    state: State<AppState>,
    security_id: Option<String>,
) -> UiResult<Vec<SecurityEventRow>> {
    let store = state.store()?;
    let events = match security_id.as_deref() {
        Some(id) => store.security_events_for(id)?,
        None => store.list_security_events()?,
    };
    let securities = store.list_securities()?;
    let actions = store.list_corporate_actions()?;

    Ok(events
        .into_iter()
        .map(|event| {
            let security = securities.iter().find(|s| s.id == event.security_id);
            SecurityEventRow {
                symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                recorded: actions.iter().any(|a| event.is_recorded_by(a)),
                event,
            }
        })
        .collect())
}

/// Writes the user's note. On a reported event only the note changes: its date and figures are
/// the provider's.
#[tauri::command]
pub fn security_event_save(
    app: AppHandle,
    state: State<AppState>,
    input: SecurityEventInput,
) -> UiResult<SecurityEvent> {
    let date = parse_date(&input.date)?;
    let note = clean(Some(input.note));
    let event = {
        let store = state.store()?;
        store.get_security(&input.security_id)?;
        let event = match input.id {
            Some(id) => {
                let existing = store.get_security_event(&id)?;
                let date = if existing.kind == SecurityEventKind::Note {
                    date
                } else {
                    existing.date
                };
                SecurityEvent {
                    date,
                    note,
                    ..existing
                }
            }
            None => SecurityEvent::note(&input.security_id, date, note.unwrap_or_default()),
        };
        store.save_security_event(&event)?;
        event
    };
    emit_changed(&app, "alerts")?;
    Ok(event)
}

#[tauri::command]
pub fn security_event_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_security_event(&id)?;
    emit_changed(&app, "alerts")
}
