//! The rates a transaction and its charges are converted at.

use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::model::Transaction;
use rust_decimal::Decimal;

/// Resolves an operation FX rate: base currency is 1, then transaction rate, then lookup.
pub(crate) fn resolve_rate(t: &Transaction, base: &str, rates: &dyn RateLookup) -> Result<Decimal> {
    if t.currency == base {
        return Ok(Decimal::ONE);
    }
    if let Some(r) = t.fx_rate_to_base {
        return Ok(r);
    }
    rates
        .rate_as_of(&t.currency, base, t.date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{}/{}", t.currency, base),
            date: t.date,
        })
}

/// A charge's own pair on the transaction's day; `fx_rate_to_base` is never lent to it (ADR-0064).
pub(crate) fn charge_rate(
    t: &Transaction,
    currency: &str,
    base: &str,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    if currency == t.currency {
        return resolve_rate(t, base, rates);
    }
    if currency == base {
        return Ok(Decimal::ONE);
    }
    rates
        .rate_as_of(currency, base, t.date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{currency}/{base}"),
            date: t.date,
        })
}

/// One transaction's charges and total, each converted the way it was actually paid.
pub(crate) struct Charges {
    pub fees_base: Decimal,
    pub taxes_base: Decimal,
    /// In the transaction's currency; a foreign charge comes back through base at same-day rates.
    pub gross_in_currency: Decimal,
    pub gross_base: Decimal,
}

impl Charges {
    pub fn of(t: &Transaction, rate: Decimal, base: &str, rates: &dyn RateLookup) -> Result<Self> {
        let fees_base = t.fees * charge_rate(t, t.fees_in(), base, rates)?;
        let taxes_base = t.taxes * charge_rate(t, t.taxes_in(), base, rates)?;
        let foreign = t.fee_currency.as_ref().map_or(Decimal::ZERO, |_| fees_base)
            + t.tax_currency.as_ref().map_or(Decimal::ZERO, |_| taxes_base);

        let sign = Decimal::from(t.kind.charge_sign());
        let mut gross_in_currency = t.gross_in_transaction_currency();
        let mut gross_base = gross_in_currency * rate;
        if !foreign.is_zero() {
            gross_base += sign * foreign;
            if !rate.is_zero() {
                gross_in_currency += sign * foreign / rate;
            }
        }
        Ok(Charges {
            fees_base,
            taxes_base,
            gross_in_currency,
            gross_base,
        })
    }
}
