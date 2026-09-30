//! Net worth: the portfolio's own total, plus what else is owned, minus what is owed.
//!
//! This is a second total and never a new value for the portfolio (ADR-0092): nothing here is
//! read by TWR, XIRR, risk, allocation or rebalancing, and an [`Asset`] has no return of its own.
//! The value of an asset on a day is the last valuation dated on or before it — a step, never an
//! interpolation — and a thing is absent before its first valuation and from its closing day on.

use super::ValueSeries;
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{Asset, AssetKind, AssetSide, AssetValue};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// One thing owned or owed, as of a reading date, with the day its figure was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetHolding {
    pub asset_id: String,
    pub name: String,
    pub kind: AssetKind,
    pub side: AssetSide,
    /// Currency of `amount`; the asset's own, which may not be the base currency.
    pub currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
    /// The day the figure is from — not the reading date. An old day means an old opinion.
    pub valued_on: NaiveDate,
    /// The asset this debt is secured by, or the debt secured on this asset. Names a
    /// relationship; no figure here depends on it.
    pub secured_by: Option<String>,
}

/// Net worth on one day, and what it is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetWorth {
    pub date: NaiveDate,
    pub base_currency: Currency,
    /// The portfolio's own total, passed in unchanged: securities plus cash.
    #[serde(with = "rust_decimal::serde::str")]
    pub investments_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub owned_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub owed_base: Decimal,
    /// `investments + owned - owed`.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
    /// How much of net worth is invested. `None` when net worth is zero or below: a share of
    /// nothing is not a share, and a share of a negative reads backwards.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub invested_share: Option<Decimal>,
    /// Owned things first, each side by size.
    pub holdings: Vec<AssetHolding>,
    /// Assets left out because no valuation of them is dated on or before `date`.
    pub not_valued_yet: Vec<String>,
}

/// One day of the net-worth line. The three parts are carried separately so the chart can stack
/// them without the frontend subtracting anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetWorthPoint {
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub investments_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub owned_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub owed_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetWorthSeries {
    pub base_currency: Currency,
    pub points: Vec<NetWorthPoint>,
}

/// Valuations of one asset, newest first, so the first row not after a date is that day's figure.
type History<'a> = HashMap<&'a str, Vec<&'a AssetValue>>;

fn history<'a>(values: &'a [AssetValue]) -> History<'a> {
    let mut by_asset: History<'a> = HashMap::new();
    for value in values {
        by_asset.entry(value.asset_id.as_str()).or_default().push(value);
    }
    for rows in by_asset.values_mut() {
        rows.sort_by_key(|value| std::cmp::Reverse(value.date));
    }
    by_asset
}

/// The figure in force on `date`: the latest one dated on or before it, or nothing at all.
fn value_as_of<'a>(history: &History<'a>, asset_id: &str, date: NaiveDate) -> Option<&'a AssetValue> {
    history
        .get(asset_id)?
        .iter()
        .find(|value| value.date <= date)
        .copied()
}

/// Whether the asset exists on `date` at all. Closing day included: a house sold on the 14th is
/// somebody else's from the 14th, and its money is in an account by then.
fn present(asset: &Asset, date: NaiveDate) -> bool {
    asset.closed_on.is_none_or(|closed| date < closed)
}

/// Net worth on `date`. `investments_base` is the portfolio's own total — read it from
/// [`super::PortfolioValuation`] and do not recompute it here.
pub fn net_worth(
    investments_base: Decimal,
    assets: &[Asset],
    values: &[AssetValue],
    base: &str,
    date: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<NetWorth> {
    let base = normalize_currency(base);
    let history = history(values);
    let mut holdings = Vec::new();
    let mut not_valued_yet = Vec::new();
    let mut owned_base = Decimal::ZERO;
    let mut owed_base = Decimal::ZERO;

    for asset in assets {
        if !present(asset, date) {
            continue;
        }
        let Some(value) = value_as_of(&history, &asset.id, date) else {
            not_valued_yet.push(asset.id.clone());
            continue;
        };
        // Converted at the reading date's rate, like every other figure in the base currency —
        // not at the rate of the day the opinion was written.
        let amount_base = rates.convert(value.amount, &asset.currency, &base, date)?;
        match asset.side() {
            AssetSide::Owned => owned_base += amount_base,
            AssetSide::Owed => owed_base += amount_base,
        }
        holdings.push(AssetHolding {
            asset_id: asset.id.clone(),
            name: asset.name.clone(),
            kind: asset.kind,
            side: asset.side(),
            currency: asset.currency.clone(),
            amount: value.amount,
            amount_base,
            valued_on: value.date,
            secured_by: asset.secured_by.clone(),
        });
    }

    holdings.sort_by(|a, b| {
        (a.side == AssetSide::Owed)
            .cmp(&(b.side == AssetSide::Owed))
            .then(b.amount_base.cmp(&a.amount_base))
            .then(a.name.cmp(&b.name))
    });

    let net_base = investments_base + owned_base - owed_base;
    Ok(NetWorth {
        date,
        base_currency: base,
        investments_base,
        owned_base,
        owed_base,
        net_base,
        invested_share: (net_base > Decimal::ZERO).then(|| investments_base / net_base),
        holdings,
        not_valued_yet,
    })
}

/// The net-worth line between two dates.
///
/// A point is placed on every day either side of the sum can move: a day of the portfolio series,
/// a valuation, a closing, and both ends of the window. The portfolio side moves daily, so a
/// window of daily data gives a daily line; the asset side steps, because between two valuations
/// there is no measurement — only the last one.
pub fn net_worth_series(
    portfolio: &ValueSeries,
    assets: &[Asset],
    values: &[AssetValue],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<NetWorthSeries> {
    let base = normalize_currency(base);
    if to < from {
        return Ok(NetWorthSeries {
            base_currency: base,
            points: Vec::new(),
        });
    }

    let mut dates: BTreeSet<NaiveDate> = [from, to].into_iter().collect();
    dates.extend(
        portfolio
            .dates
            .iter()
            .copied()
            .filter(|d| (from..=to).contains(d)),
    );
    dates.extend(values.iter().map(|v| v.date).filter(|d| (from..=to).contains(d)));
    dates.extend(
        assets
            .iter()
            .filter_map(|a| a.closed_on)
            .filter(|d| (from..=to).contains(d)),
    );

    let points = dates
        .into_iter()
        .map(|date| {
            let reading = net_worth(invested_on(portfolio, date), assets, values, &base, date, rates)?;
            Ok(NetWorthPoint {
                date,
                investments_base: reading.investments_base,
                owned_base: reading.owned_base,
                owed_base: reading.owed_base,
                net_base: reading.net_base,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(NetWorthSeries {
        base_currency: base,
        points,
    })
}

/// The portfolio's value on `date`, or on the last day of the series before it. Zero before the
/// series starts: there was nothing invested yet, which is not the same as a gap.
fn invested_on(portfolio: &ValueSeries, date: NaiveDate) -> Decimal {
    match portfolio.dates.binary_search(&date) {
        Ok(i) => portfolio.total_value_base[i],
        Err(0) => Decimal::ZERO,
        Err(i) => portfolio.total_value_base[i - 1],
    }
}

#[cfg(test)]
mod tests;
