//! Things owned and owed beside the portfolio, and the net-worth reading they make.
//!
//! Not scoped: an asset is not an account, so the picker narrows none of this (ADR-0092). Nothing
//! here reaches the portfolio's own figures — net worth is a second total, never a new one.

use crate::commands::alerts::today;
use crate::commands::parse_date;
use crate::commands::performance::date_range;
use crate::commands::portfolio::{require_currency, require_text};
use crate::commands::transactions::decimal;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::calc::{NetWorth, NetWorthSeries};
use sq_core::prelude::*;
use tauri::{AppHandle, State};

/// The reading, plus every asset the portfolio has. The two are not the same list: an asset with
/// no valuation on or before the reading date is absent from the reading and still needs a row on
/// the screen, or there would be no way to give it its first figure.
#[derive(Debug, Serialize)]
pub struct NetWorthData {
    pub reading: NetWorth,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct AssetInput {
    pub id: Option<String>,
    pub name: String,
    /// One of `AssetKind` as the wire spells it: `PROPERTY`, `MORTGAGE`, …
    pub kind: String,
    pub currency: String,
    pub secured_by: Option<String>,
    pub note: Option<String>,
    pub closed_on: Option<String>,
    /// Percent a year, as the user typed it: `3.45` is 3.45%. Only a debt has one.
    pub rate: Option<String>,
    pub monthly_payment: Option<String>,
    pub ends_on: Option<String>,
    /// A first valuation, so saying what a thing is and what it is worth is one step. Left out
    /// when the asset already exists: editing a name must not rewrite today's figure.
    pub amount: Option<String>,
    pub valued_on: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssetValueInput {
    pub asset_id: String,
    pub date: String,
    pub amount: String,
    pub note: Option<String>,
}

#[tauri::command]
pub fn net_worth(state: State<AppState>, date: String) -> UiResult<NetWorthData> {
    let as_of = parse_date(&date)?;
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let analytics = PortfolioAnalytics::new(&store, &portfolio)?;

    Ok(NetWorthData {
        reading: analytics.net_worth(as_of)?,
        assets: store.list_assets(&portfolio.id)?,
    })
}

/// The net-worth line. Its portfolio side is the same daily series the performance screen draws.
#[tauri::command]
pub fn net_worth_series(state: State<AppState>, from: String, to: String) -> UiResult<NetWorthSeries> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let analytics = PortfolioAnalytics::new(&store, &portfolio)?;

    Ok(analytics.net_worth_series(range.from, range.to)?)
}

/// Every figure ever written for one asset, oldest first.
#[tauri::command]
pub fn asset_values(state: State<AppState>, asset_id: String) -> UiResult<Vec<AssetValue>> {
    Ok(state.store()?.asset_values(&asset_id)?)
}

#[tauri::command]
pub fn asset_save(app: AppHandle, state: State<AppState>, input: AssetInput) -> UiResult<Asset> {
    let asset = {
        let store = state.store()?;
        let portfolio = state.portfolio()?.clone();
        let kind = AssetKind::parse(&input.kind).map_err(UiError::from)?;
        let schedule = schedule_of(&input)?;
        let mut asset = match &input.id {
            Some(id) => store.get_asset(id)?,
            None => Asset::new(&input.name, kind, &input.currency),
        };
        asset.name = require_text(&input.name, "name")?;
        asset.kind = kind;
        asset.currency = require_currency(&input.currency)?;
        asset.secured_by = input.secured_by.filter(|id| !id.trim().is_empty());
        asset.note = input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        asset.closed_on = input.closed_on.as_deref().map(parse_date).transpose()?;
        asset.schedule = schedule;
        store.save_asset(&portfolio.id, &asset)?;

        // A first valuation only on the way in, and only if the form gave one.
        if input.id.is_none()
            && let Some(amount) = decimal(input.amount.as_deref(), "value")?
        {
            let day = input.valued_on.as_deref().map(parse_date).transpose()?;
            store.save_asset_value(&AssetValue::new(&asset.id, day.unwrap_or_else(today), amount))?;
        }
        asset
    };

    emit_changed(&app, "assets")?;
    Ok(asset)
}

#[tauri::command]
pub fn asset_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_asset(&id)?;
    emit_changed(&app, "assets")
}

/// Writes what a thing is worth on one day. The same day twice replaces that day's figure.
#[tauri::command]
pub fn asset_value_save(app: AppHandle, state: State<AppState>, input: AssetValueInput) -> UiResult<()> {
    {
        let store = state.store()?;
        // Reading it first turns a stale id into "not found" rather than a foreign-key failure.
        store.get_asset(&input.asset_id)?;
        let amount = decimal(Some(&input.amount), "value")?
            .ok_or_else(|| UiError::invalid("a valuation needs an amount"))?;
        let mut value = AssetValue::new(&input.asset_id, parse_date(&input.date)?, amount);
        value.note = input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        store.save_asset_value(&value)?;
    }

    emit_changed(&app, "assets")
}

#[tauri::command]
pub fn asset_value_delete(
    app: AppHandle,
    state: State<AppState>,
    asset_id: String,
    date: String,
) -> UiResult<()> {
    state.store()?.delete_asset_value(&asset_id, parse_date(&date)?)?;
    emit_changed(&app, "assets")
}

/// The debt's schedule, or nothing. A rate with no payment is half an answer, so both are
/// required together; the model refuses one on a thing owned.
fn schedule_of(input: &AssetInput) -> UiResult<Option<Amortization>> {
    let rate = decimal(input.rate.as_deref(), "interest rate")?;
    let payment = decimal(input.monthly_payment.as_deref(), "monthly payment")?;
    match (rate, payment) {
        (None, None) => Ok(None),
        (Some(rate), Some(monthly_payment)) => Ok(Some(Amortization {
            // The form asks for a percent; every rate in the calculations is a fraction.
            rate: sq_core::calc::percent_to_rate(rate)
                .ok_or_else(|| UiError::invalid("the interest rate is not a number"))?,
            monthly_payment,
            ends_on: input.ends_on.as_deref().map(parse_date).transpose()?,
        })),
        _ => Err(UiError::invalid(
            "a schedule needs both an interest rate and a monthly payment",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn a_schedule_needs_both_halves() {
        let base = AssetInput {
            id: None,
            name: "Mortgage".into(),
            kind: "MORTGAGE".into(),
            currency: "EUR".into(),
            secured_by: None,
            note: None,
            closed_on: None,
            rate: Some("3.45".into()),
            monthly_payment: None,
            ends_on: None,
            amount: None,
            valued_on: None,
        };
        assert!(schedule_of(&base).is_err());

        let both = AssetInput {
            monthly_payment: Some("1100".into()),
            ..base
        };
        let schedule = schedule_of(&both).unwrap().unwrap();
        // 3.45 typed is 0.0345 stored, like every other rate the forms ask for as a percent.
        assert_eq!(schedule.rate, Decimal::new(345, 4));
        assert_eq!(schedule.monthly_payment, Decimal::new(1100, 0));
    }

    #[test]
    fn no_schedule_at_all_is_not_an_error() {
        let plain = AssetInput {
            id: None,
            name: "Flat".into(),
            kind: "PROPERTY".into(),
            currency: "EUR".into(),
            secured_by: None,
            note: None,
            closed_on: None,
            rate: None,
            monthly_payment: None,
            ends_on: None,
            amount: Some("400000".into()),
            valued_on: None,
        };
        assert!(schedule_of(&plain).unwrap().is_none());
    }
}
