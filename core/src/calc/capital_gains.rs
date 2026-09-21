use super::{Holdings, RealizedGain};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Realized result rolled up over disposals; fees and taxes stay separate for reporting,
/// while `gain_base` remains the net total.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizedSummary {
    /// Number of disposals in the group.
    pub disposals: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub proceeds_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    /// Proceeds - fees - taxes - cost basis.
    #[serde(with = "rust_decimal::serde::str")]
    pub gain_base: Decimal,
    /// The exchange rate's share of `gain_base`, each disposal at its own rate — see ADR-0028.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub currency_gain_base: Decimal,
}

impl RealizedSummary {
    /// Result against the cost of what was sold. `None` when there is no cost to divide by —
    /// shares delivered in for free and then sold are not an infinite return.
    pub fn return_on_cost(&self) -> Option<Decimal> {
        return_on_cost(self.gain_base, self.cost_base)
    }

    /// The result the instruments made, with the currency move taken out.
    pub fn instrument_gain_base(&self) -> Decimal {
        self.gain_base - self.currency_gain_base
    }

    fn add(&mut self, g: &RealizedGain) {
        self.disposals += 1;
        self.proceeds_base += g.proceeds_base;
        self.cost_base += g.cost_base;
        self.fees_base += g.fees_base;
        self.taxes_base += g.taxes_base;
        self.gain_base += g.gain_base;
        self.currency_gain_base += g.currency_gain_base;
    }
}

/// Realized result as a fraction of the cost it came from; `None` when that cost is zero.
pub fn return_on_cost(gain_base: Decimal, cost_base: Decimal) -> Option<Decimal> {
    if cost_base.is_zero() {
        return None;
    }
    Some(gain_base / cost_base)
}

/// Disposals in `[from, to]`, inclusive — the same window rule as [`super::income_between`].
pub fn realized_between(holdings: &Holdings, from: NaiveDate, to: NaiveDate) -> Vec<RealizedGain> {
    holdings
        .realized
        .iter()
        .filter(|g| g.date >= from && g.date <= to)
        .cloned()
        .collect()
}

/// Realized gains by calendar year of **disposal** (not purchase) — tax liability arises on sale.
pub fn capital_gains_by_year(items: &[RealizedGain]) -> BTreeMap<i32, RealizedSummary> {
    group(items, |g| g.date.year())
}

pub fn capital_gains_by_security(items: &[RealizedGain]) -> BTreeMap<String, RealizedSummary> {
    group(items, |g| g.security_id.clone())
}

/// Per security within one year, for a tax filing's "which trades made up 2024's result".
pub fn capital_gains_by_year_and_security(
    items: &[RealizedGain],
) -> BTreeMap<(i32, String), RealizedSummary> {
    group(items, |g| (g.date.year(), g.security_id.clone()))
}

pub fn capital_gains_total(items: &[RealizedGain]) -> RealizedSummary {
    let mut total = RealizedSummary::default();
    for g in items {
        total.add(g);
    }
    total
}

/// Includes every disposal, including `DeliveryOutbound` — it also realizes P/L, and skipping
/// it would lose gains accrued before the transfer out.
fn group<K: Ord, F: Fn(&RealizedGain) -> K>(items: &[RealizedGain], key: F) -> BTreeMap<K, RealizedSummary> {
    let mut out: BTreeMap<K, RealizedSummary> = BTreeMap::new();
    for g in items {
        out.entry(key(g)).or_default().add(g);
    }
    out
}
