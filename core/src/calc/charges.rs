use super::{ChargeRecord, Holdings, resolve_rate};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{Transaction, TransactionKind};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Charges rolled up over a group of operations. Fees and taxes stay separate fields —
/// different lines on a tax return, and a fee can be shopped around while a tax can't.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeSummary {
    /// Includes refunds.
    pub count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
}

impl ChargeSummary {
    fn add(&mut self, c: &ChargeRecord) {
        self.count += 1;
        match c.kind {
            TransactionKind::Fee | TransactionKind::FeeRefund => self.fees_base += c.amount_base,
            _ => self.taxes_base += c.amount_base,
        }
    }

    pub fn total_base(&self) -> Decimal {
        self.fees_base + self.taxes_base
    }
}

/// Charges in `[from, to]`, inclusive — the same window rule as [`super::income_between`].
pub fn charges_between(holdings: &Holdings, from: NaiveDate, to: NaiveDate) -> Vec<ChargeRecord> {
    holdings
        .charges
        .iter()
        .filter(|c| c.date >= from && c.date <= to)
        .cloned()
        .collect()
}

/// Charges by calendar year. [`Holdings::charges`] already excludes fees baked into a
/// trade's cost basis and dividend withholding (see CLAUDE.md) — this is a plain rollup.
pub fn charges_by_year(items: &[ChargeRecord]) -> BTreeMap<i32, ChargeSummary> {
    group(items, |c| c.date.year())
}

pub fn charges_by_kind(items: &[ChargeRecord]) -> BTreeMap<TransactionKind, ChargeSummary> {
    group(items, |c| c.kind)
}

/// Where the costs are billed. Every charge sits on an account, so unlike a security
/// breakdown this one loses no row.
pub fn charges_by_account(items: &[ChargeRecord]) -> BTreeMap<String, ChargeSummary> {
    group(items, |c| c.account_id.clone())
}

/// Charges attributable to one instrument; account-level charges have no key and drop out.
pub fn charges_by_security(items: &[ChargeRecord]) -> BTreeMap<String, ChargeSummary> {
    let mut out: BTreeMap<String, ChargeSummary> = BTreeMap::new();
    for c in items {
        if let Some(sid) = &c.security_id {
            out.entry(sid.clone()).or_default().add(c);
        }
    }
    out
}

pub fn charges_total(items: &[ChargeRecord]) -> ChargeSummary {
    let mut total = ChargeSummary::default();
    for c in items {
        total.add(c);
    }
    total
}

/// Every fee and tax actually paid in `[from, to]`, trade commissions included.
///
/// The rollups above deliberately see standalone Fee/Tax operations only: a buy commission is
/// already inside the cost basis and counting it twice would overstate expenses. A cost *rate*
/// asks the opposite question — what did holding and trading this portfolio cost — so it must
/// count the commission that a purchase buried in the cost basis and the tax withheld from a
/// dividend. This is a separate function rather than a flag because the two answers are both
/// right and neither may be silently substituted for the other.
pub fn costs_paid(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<ChargeSummary> {
    let mut total = ChargeSummary::default();
    for entry in cost_entries(transactions, base, from, to, rates)? {
        total.count += 1;
        total.fees_base += entry.fees_base;
        total.taxes_base += entry.taxes_base;
    }
    Ok(total)
}

/// The same costs attributed to the instrument that incurred them; an account-level fee has
/// no instrument and drops out, exactly as [`charges_by_security`] drops one.
pub fn costs_paid_by_security(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<BTreeMap<String, ChargeSummary>> {
    let mut out: BTreeMap<String, ChargeSummary> = BTreeMap::new();
    for entry in cost_entries(transactions, base, from, to, rates)? {
        let Some(sid) = entry.security_id else { continue };
        let summary = out.entry(sid).or_default();
        summary.count += 1;
        summary.fees_base += entry.fees_base;
        summary.taxes_base += entry.taxes_base;
    }
    Ok(out)
}

/// One transaction's cost side in base currency.
struct CostEntry {
    security_id: Option<String>,
    fees_base: Decimal,
    taxes_base: Decimal,
}

/// A standalone Fee/Tax operation carries its amount in `amount`, everything else in
/// `fees`/`taxes`; a refund is the same operation with the sign turned around.
fn cost_entries(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<Vec<CostEntry>> {
    let mut out = Vec::new();
    for t in transactions.iter().filter(|t| t.date >= from && t.date <= to) {
        let rate = resolve_rate(t, base, rates)?;
        // Each charge at the rate of the currency it was billed in, not of the trade's.
        let mut fees = t.fees * super::holdings::charge_rate(t, t.fees_in(), base, rates)?;
        let mut taxes = t.taxes * super::holdings::charge_rate(t, t.taxes_in(), base, rates)?;
        match t.kind {
            TransactionKind::Fee => fees += t.amount * rate,
            TransactionKind::FeeRefund => fees -= t.amount * rate,
            TransactionKind::Tax => taxes += t.amount * rate,
            TransactionKind::TaxRefund => taxes -= t.amount * rate,
            _ => {}
        }
        if fees.is_zero() && taxes.is_zero() {
            continue;
        }
        out.push(CostEntry {
            security_id: t.security_id.clone(),
            fees_base: fees,
            taxes_base: taxes,
        });
    }
    Ok(out)
}

fn group<K: Ord, F: Fn(&ChargeRecord) -> K>(items: &[ChargeRecord], key: F) -> BTreeMap<K, ChargeSummary> {
    let mut out: BTreeMap<K, ChargeSummary> = BTreeMap::new();
    for c in items {
        out.entry(key(c)).or_default().add(c);
    }
    out
}
