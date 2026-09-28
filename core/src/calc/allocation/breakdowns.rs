//! One-level allocations: by currency, by security, by account.

use super::{Allocation, AllocationBucket, Holdings, HoldingsOptions, PortfolioValuation, push_rest, share};
use crate::calc::holdings::{Event, ordered_events};
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::PriceLookup;
use crate::model::{Account, Security, Transaction};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};

/// Allocates securities by quote currency and cash by its holding currency.
pub fn allocation_by_currency(
    valuation: &PortfolioValuation,
    holdings: &Holdings,
    date: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<Allocation> {
    let mut by_currency: BTreeMap<Currency, Decimal> = BTreeMap::new();
    for p in &valuation.positions {
        *by_currency.entry(p.currency.clone()).or_default() += p.market_value_base;
    }
    for (currency, amount) in &holdings.cash {
        if amount.is_zero() {
            continue;
        }
        *by_currency.entry(currency.clone()).or_default() +=
            rates.convert(*amount, currency, &valuation.base_currency, date)?;
    }

    let total = valuation.total_value_base;
    let buckets = by_currency
        .into_iter()
        .map(|(currency, value)| AllocationBucket {
            key: currency.clone(),
            label: currency,
            value_base: value,
            weight: share(value, total),
            children: Vec::new(),
        })
        .collect();
    Ok(Allocation {
        total_base: total,
        buckets,
    })
}

/// Allocates by security, with cash in a separate bucket.
pub fn allocation_by_security(valuation: &PortfolioValuation, securities: &[Security]) -> Allocation {
    let names: HashMap<&str, &Security> = securities.iter().map(|s| (s.id.as_str(), s)).collect();
    let total = valuation.total_value_base;
    let mut buckets: Vec<AllocationBucket> = valuation
        .positions
        .iter()
        .map(|p| AllocationBucket {
            key: p.security_id.clone(),
            label: names
                .get(p.security_id.as_str())
                .map(|s| s.symbol.clone())
                .unwrap_or_else(|| p.security_id.clone()),
            value_base: p.market_value_base,
            weight: share(p.market_value_base, total),
            children: Vec::new(),
        })
        .collect();
    push_rest(&mut buckets, Decimal::ZERO, valuation.cash_base, total);
    Allocation {
        total_base: total,
        buckets,
    }
}

/// Allocates by account using per-account quantities and cash; cost basis remains portfolio-wide.
// Arguments represent independent calculation inputs; keep the public API explicit.
#[allow(clippy::too_many_arguments)]
pub fn allocation_by_account(
    transactions: &[Transaction],
    accounts: &[Account],
    securities: &[Security],
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Allocation> {
    let base = normalize_currency(base);
    let by_id: HashMap<&str, &Security> = securities.iter().map(|s| (s.id.as_str(), s)).collect();
    // Depot trades settle cash on the linked cash account, not on the depot.
    let settlement = crate::calc::balances::settlement_accounts(accounts);

    let mut quantities: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let mut cash: BTreeMap<(String, Currency), Decimal> = BTreeMap::new();

    for event in ordered_events(transactions, options.corporate_actions) {
        if event.date() > date {
            break;
        }
        match event {
            Event::Action(action) => {
                let factor = action.quantity_factor()?;
                for ((_, security_id), quantity) in quantities.iter_mut() {
                    if *security_id == action.security_id {
                        *quantity = crate::money::fit_quantity(*quantity * factor);
                    }
                }
            }
            Event::Tx(t) => {
                let sign = Decimal::from(t.kind.quantity_sign());
                if !sign.is_zero()
                    && let Some(sid) = &t.security_id
                {
                    *quantities.entry((t.account_id.clone(), sid.clone())).or_default() += sign * t.quantity;
                }
                let delta = t.cash_delta();
                let legs = t.foreign_charge_legs();
                if !delta.is_zero() || !legs.is_empty() {
                    let account_id = settlement
                        .get(t.account_id.as_str())
                        .map(|id| (*id).to_string())
                        .unwrap_or_else(|| t.account_id.clone());
                    *cash.entry((account_id.clone(), t.currency.clone())).or_default() += delta;
                    // A charge billed elsewhere is a cash subject of its own currency.
                    for (currency, amount) in legs {
                        *cash.entry((account_id.clone(), currency)).or_default() += amount;
                    }
                }
            }
        }
    }

    let mut values: BTreeMap<String, Decimal> = BTreeMap::new();
    for ((account_id, security_id), quantity) in &quantities {
        if quantity.is_zero() {
            continue;
        }
        // A transaction security must exist in the supplied reference data.
        if !by_id.contains_key(security_id.as_str()) {
            return Err(Error::NotFound(format!("security {security_id}")));
        }
        let price = prices
            .price_as_of(security_id, date)?
            .ok_or_else(|| Error::MissingMarketData {
                kind: "price",
                key: security_id.clone(),
                date,
            })?;
        // Use the quote currency returned by the provider, not stale reference data.
        let value = rates.convert(*quantity * price.close, &price.currency, &base, date)?;
        *values.entry(account_id.clone()).or_default() += value;
    }
    for ((account_id, currency), amount) in &cash {
        if amount.is_zero() {
            continue;
        }
        *values.entry(account_id.clone()).or_default() += rates.convert(*amount, currency, &base, date)?;
    }

    let names: HashMap<&str, &Account> = accounts.iter().map(|a| (a.id.as_str(), a)).collect();
    let total: Decimal = values.values().sum();
    let buckets = values
        .into_iter()
        .map(|(account_id, value)| AllocationBucket {
            label: names
                .get(account_id.as_str())
                .map(|a| a.name.clone())
                .unwrap_or_else(|| account_id.clone()),
            key: account_id,
            value_base: value,
            weight: share(value, total),
            children: Vec::new(),
        })
        .collect();
    Ok(Allocation {
        total_base: total,
        buckets,
    })
}
