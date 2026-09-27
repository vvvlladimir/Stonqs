//! Transactions -> positions, cash and flows, with no market prices. Built once per series
//! through `HoldingsBuilder`, never per day.

mod apply;
mod builder;
mod charges;
mod lots;
mod records;

pub(crate) use builder::{Event, HoldingsBuilder, ordered_events};
pub(crate) use charges::{charge_rate, resolve_rate};
pub use records::{CashFlow, ChargeRecord, IncomeRecord, RealizedGain};

use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{CorporateAction, CostBasisMethod, Position, Transaction};
use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Position-building options kept together and cheap to copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct HoldingsOptions<'a> {
    pub cost_basis: CostBasisMethod,
    /// Splits and other security events; an empty slice means none.
    pub corporate_actions: &'a [CorporateAction],
}

impl<'a> HoldingsOptions<'a> {
    pub fn with_cost_basis(mut self, method: CostBasisMethod) -> Self {
        self.cost_basis = method;
        self
    }

    pub fn with_corporate_actions(mut self, actions: &'a [CorporateAction]) -> Self {
        self.corporate_actions = actions;
        self
    }
}

/// Portfolio state after applying transactions through a date; it contains no market prices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Holdings {
    /// Positions keyed by security ID; `BTreeMap` keeps output deterministic.
    pub positions: BTreeMap<String, Position>,
    /// Cash by currency; simultaneous positive and negative balances are valid.
    pub cash: BTreeMap<Currency, Decimal>,
    /// Chronological external flows in base currency.
    pub external_flows: Vec<CashFlow>,
    /// Disposals with results for realized-gain reports.
    #[serde(default)]
    pub realized: Vec<RealizedGain>,
    /// Income events for dividend and interest reports.
    #[serde(default)]
    pub income: Vec<IncomeRecord>,
    /// Standalone fees and taxes for expense reports.
    #[serde(default)]
    pub charges: Vec<ChargeRecord>,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    /// The share of `realized_pnl_base` that came from the exchange rate.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub realized_currency_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_base: Decimal,
    /// Net interest: received minus paid.
    #[serde(with = "rust_decimal::serde::str")]
    pub interest_base: Decimal,
    /// Standalone fees; trade fees are already in cost or proceeds.
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
}

impl Holdings {
    /// Converts all non-zero cash balances to base currency at `date`.
    pub fn cash_in_base(&self, base: &str, date: NaiveDate, rates: &dyn RateLookup) -> Result<Decimal> {
        let mut total = Decimal::ZERO;
        for (currency, amount) in &self.cash {
            if amount.is_zero() {
                continue;
            }
            total += rates.convert(*amount, currency, base, date)?;
        }
        Ok(total)
    }

    /// Open positions with non-zero quantity.
    pub fn open_positions(&self) -> impl Iterator<Item = (&String, &Position)> {
        self.positions.iter().filter(|(_, p)| !p.is_closed())
    }
}

/// Builds FIFO holdings without corporate actions; convenience wrapper for simple callers.
pub fn build_holdings(transactions: &[Transaction], base: &str, rates: &dyn RateLookup) -> Result<Holdings> {
    build_holdings_with(transactions, base, rates, HoldingsOptions::default())
}

/// Builds holdings with a cost-basis method and corporate actions.
/// A transaction's recorded FX rate takes precedence over the lookup.
pub fn build_holdings_with(
    transactions: &[Transaction],
    base: &str,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Holdings> {
    let mut builder = HoldingsBuilder::new(transactions, base, rates, options);
    for event in ordered_events(transactions, options.corporate_actions) {
        builder.apply(event)?;
    }
    builder.finish()
}

/// Link ids carried by at least two rows. A lone leg is money crossing the portfolio boundary:
/// brokers print per-row ids under the same header, and believing one would hide every flow.
pub(crate) fn paired_links(transactions: &[Transaction]) -> HashSet<&str> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for t in transactions {
        if let Some(link) = t.link_id.as_deref() {
            *seen.entry(link).or_insert(0) += 1;
        }
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(link, _)| link)
        .collect()
}

#[cfg(test)]
mod tests;
