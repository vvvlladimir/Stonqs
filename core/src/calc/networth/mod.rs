//! Net worth: the portfolio's own total, plus what else is owned, minus what is owed.
//!
//! This is a second total and never a new value for the portfolio (ADR-0092): nothing here is
//! read by TWR, XIRR, risk, allocation or rebalancing, and an [`Asset`] has no return of its own.
//! The value of an asset on a day is the last valuation dated on or before it — a step, never an
//! interpolation — and a thing is absent before its first valuation and from its closing day on.

mod after_tax;
mod payoff;

pub use after_tax::{AfterTax, after_tax};
pub use payoff::DebtPayoff;

use super::ValueSeries;
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{Asset, AssetKind, AssetSide, AssetValue};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// When a figure is old enough to say so. Half a year: property and cars are re-valued about
/// once a year, so a shorter fuse would mark almost everything and mean nothing, while a longer
/// one lets a figure go a whole cycle without anybody noticing.
pub const STALE_AFTER_DAYS: i64 = 180;

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
    /// Days from `valued_on` to the reading date.
    pub days_old: i64,
    /// Whether that age is worth pointing at; see [`STALE_AFTER_DAYS`]. An old figure is not
    /// wrong, and only the owner can say whether it still holds.
    pub stale: bool,
    /// What the last revaluation changed, both figures converted at the reading date's rate so
    /// the difference is the revaluation alone and not the currency's move. `None` for a thing
    /// with only one figure: there is nothing to compare it with.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub change_base: Option<Decimal>,
    /// The day the figure `change_base` measures from.
    pub changed_since: Option<NaiveDate>,
    /// The asset this debt is secured by, or the debt secured on this asset. Names a
    /// relationship; no figure here depends on it.
    pub secured_by: Option<String>,
    /// For a thing owned: what is owed against it, summed over the debts that name it.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub secured_debt_base: Option<Decimal>,
    /// For a thing owned with a debt against it: what is left after that debt. A rendering of
    /// two figures already here (ADR-0059), kept in one place so both sides read it the same.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub equity_base: Option<Decimal>,
    /// For a debt with a schedule: when it ends and what it still costs.
    pub payoff: Option<DebtPayoff>,
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
    /// Everything owed over everything owned, investments included. `None` when there is nothing
    /// owned: a debt against no assets is not a ratio, it is just a debt.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub debt_to_assets: Option<Decimal>,
    /// Owned things first, each side by size.
    pub holdings: Vec<AssetHolding>,
    /// How many holdings carry a figure older than [`STALE_AFTER_DAYS`].
    #[serde(default)]
    pub stale_count: usize,
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

/// The figure written before the one in force, which is what a change is measured against.
fn value_before<'a>(history: &History<'a>, asset_id: &str, date: NaiveDate) -> Option<&'a AssetValue> {
    history
        .get(asset_id)?
        .iter()
        .filter(|value| value.date <= date)
        .nth(1)
        .copied()
}

/// The oldest figure of all, which is where progress on a debt is measured from. Not narrowed by
/// the reading date: the first figure is the first figure whatever day is being read.
fn first_value<'a>(history: &History<'a>, asset_id: &str) -> Option<&'a AssetValue> {
    history.get(asset_id)?.last().copied()
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
        // Both figures at one rate, so the difference is the revaluation and nothing else.
        let previous = value_before(&history, &asset.id, date);
        let previous_base = previous
            .map(|before| rates.convert(before.amount, &asset.currency, &base, date))
            .transpose()?;
        let days_old = (date - value.date).num_days();

        holdings.push(AssetHolding {
            asset_id: asset.id.clone(),
            name: asset.name.clone(),
            kind: asset.kind,
            side: asset.side(),
            currency: asset.currency.clone(),
            amount: value.amount,
            amount_base,
            valued_on: value.date,
            days_old,
            stale: days_old > STALE_AFTER_DAYS,
            change_base: previous_base.map(|before| amount_base - before),
            changed_since: previous.map(|before| before.date),
            secured_by: asset.secured_by.clone(),
            secured_debt_base: None,
            equity_base: None,
            payoff: asset.schedule.as_ref().map(|schedule| {
                // Progress needs two figures to be progress at all. With only one, the first
                // figure *is* today's, and reporting nothing beats reporting a 0% that can never
                // move until a second valuation is written.
                let first = first_value(&history, &asset.id)
                    .filter(|first| first.date != value.date)
                    .map(|first| first.amount);
                payoff::payoff(value.amount, first, schedule, date)
            }),
        });
    }

    attach_equity(&mut holdings);

    holdings.sort_by(|a, b| {
        (a.side == AssetSide::Owed)
            .cmp(&(b.side == AssetSide::Owed))
            .then(b.amount_base.cmp(&a.amount_base))
            .then(a.name.cmp(&b.name))
    });

    let net_base = investments_base + owned_base - owed_base;
    let assets_base = investments_base + owned_base;
    Ok(NetWorth {
        date,
        base_currency: base,
        investments_base,
        owned_base,
        owed_base,
        net_base,
        invested_share: (net_base > Decimal::ZERO).then(|| investments_base / net_base),
        debt_to_assets: (assets_base > Decimal::ZERO).then(|| owed_base / assets_base),
        stale_count: holdings.iter().filter(|holding| holding.stale).count(),
        holdings,
        not_valued_yet,
    })
}

/// The two sides on `date`, and nothing else. What a full [`NetWorth`] carries besides them —
/// the sorted holdings, the equity each debt leaves, the month-by-month amortization of every
/// schedule — is per-reading detail no point of a line ever shows, and a line is drawn over
/// thousands of days.
fn sides_on(
    assets: &[Asset],
    history: &History<'_>,
    base: &str,
    date: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<(Decimal, Decimal)> {
    let mut owned_base = Decimal::ZERO;
    let mut owed_base = Decimal::ZERO;
    for asset in assets.iter().filter(|asset| present(asset, date)) {
        let Some(value) = value_as_of(history, &asset.id, date) else {
            continue;
        };
        let amount_base = rates.convert(value.amount, &asset.currency, base, date)?;
        match asset.side() {
            AssetSide::Owned => owned_base += amount_base,
            AssetSide::Owed => owed_base += amount_base,
        }
    }
    Ok((owned_base, owed_base))
}

/// What a debt leaves of the thing it is secured on. Several debts may name one asset — a
/// mortgage and a renovation loan on the same flat — so they are summed rather than matched one
/// to one, and an asset nothing is secured on keeps no equity figure at all: repeating its own
/// value under a second name would only invite the reader to add them up.
fn attach_equity(holdings: &mut [AssetHolding]) {
    let debts: Vec<(String, Decimal)> = holdings
        .iter()
        .filter(|holding| holding.side == AssetSide::Owed)
        .filter_map(|holding| holding.secured_by.clone().map(|on| (on, holding.amount_base)))
        .collect();

    for holding in holdings.iter_mut().filter(|h| h.side == AssetSide::Owned) {
        let owed: Decimal = debts
            .iter()
            .filter(|(on, _)| *on == holding.asset_id)
            .map(|(_, amount)| *amount)
            .sum();
        if owed > Decimal::ZERO {
            holding.secured_debt_base = Some(owed);
            holding.equity_base = Some(holding.amount_base - owed);
        }
    }
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

    // Sorted once for the whole line rather than once per day.
    let history = history(values);
    let points = dates
        .into_iter()
        .map(|date| {
            let investments_base = invested_on(portfolio, date);
            let (owned_base, owed_base) = sides_on(assets, &history, &base, date, rates)?;
            Ok(NetWorthPoint {
                date,
                investments_base,
                owned_base,
                owed_base,
                net_base: investments_base + owned_base - owed_base,
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
