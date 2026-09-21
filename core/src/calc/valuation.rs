use super::Holdings;
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::PriceLookup;
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Valuation of one position, retaining both quote-currency and base-currency values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionValuation {
    pub security_id: String,
    /// Quote currency for `price` and `market_value`.
    pub currency: Currency,
    /// Settlement currency of `cost_basis`; it may differ from the quote currency.
    pub cost_currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Price on the date, or the latest known price before it.
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    /// Quote-currency to base-currency rate for the same date.
    #[serde(with = "rust_decimal::serde::str")]
    pub fx_rate: Decimal,
    /// Settlement-currency to base-currency rate for the same date. Equal to `fx_rate` only
    /// when the instrument is quoted in the currency it was paid for.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub cost_fx_rate: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub market_value: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub market_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub unrealized_pnl_base: Decimal,
    /// The part of `unrealized_pnl_base` the exchange rate made: what the money put in is worth
    /// today minus what it was worth when it was paid. See ADR-0028.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub currency_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    /// Quantity step observed in transactions; rebalancing needs the broker's actual step.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub observed_quantity_step: Option<Decimal>,
}

impl PositionValuation {
    /// What the instrument itself earned, with the currency move taken out.
    pub fn instrument_gain_base(&self) -> Decimal {
        self.unrealized_pnl_base - self.currency_gain_base
    }
}

/// Whole-portfolio valuation in the base currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioValuation {
    pub date: NaiveDate,
    pub base_currency: Currency,
    pub positions: Vec<PositionValuation>,
    #[serde(with = "rust_decimal::serde::str")]
    pub securities_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_base: Decimal,
    /// Securities plus cash; this is the value used by TWR.
    #[serde(with = "rust_decimal::serde::str")]
    pub total_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub unrealized_pnl_base: Decimal,
    /// The currency's share of `unrealized_pnl_base`, summed over open positions.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub currency_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    /// The currency's share of `realized_pnl_base`, fixed at each disposal's own rate.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub realized_currency_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_base: Decimal,
    /// Net account interest: received minus paid.
    #[serde(with = "rust_decimal::serde::str")]
    pub interest_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
}

impl PortfolioValuation {
    /// What the instruments themselves are up, with the currency move taken out.
    pub fn instrument_gain_base(&self) -> Decimal {
        self.unrealized_pnl_base - self.currency_gain_base
    }

    /// Total result: unrealized + realized + dividends + interest - expenses.
    pub fn total_pnl_base(&self) -> Decimal {
        self.unrealized_pnl_base + self.realized_pnl_base + self.dividends_base + self.interest_base
            - self.fees_base
            - self.taxes_base
    }
}

/// Values holdings as `quantity * price(date) * fx_rate(date)`.
/// Closed positions need no quote; their realized P/L remains in `Holdings`.
pub fn value_holdings(
    holdings: &Holdings,
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<PortfolioValuation> {
    let base = normalize_currency(base);
    let mut positions = Vec::new();
    let mut securities_value_base = Decimal::ZERO;
    let mut cost_basis_base = Decimal::ZERO;
    let mut unrealized_total = Decimal::ZERO;
    let mut currency_total = Decimal::ZERO;

    for (security_id, p) in &holdings.positions {
        if p.is_closed() {
            continue;
        }
        let price = prices
            .price_as_of(security_id, date)?
            .ok_or_else(|| Error::MissingMarketData {
                kind: "price",
                key: security_id.clone(),
                date,
            })?;
        // Convert the quote currency, not the broker's settlement currency.
        let fx_rate =
            rates
                .rate_as_of(&price.currency, &base, date)?
                .ok_or_else(|| Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{}", price.currency, base),
                    date,
                })?;

        // The settlement currency is a second rate: the cost was paid in it, the quote is not
        // necessarily in it. Reuse the quote rate when they coincide rather than asking twice.
        let cost_fx_rate = if p.cost_currency == base {
            Decimal::ONE
        } else if p.cost_currency == price.currency {
            fx_rate
        } else {
            rates
                .rate_as_of(&p.cost_currency, &base, date)?
                .ok_or_else(|| Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{}", p.cost_currency, base),
                    date,
                })?
        };

        let market_value = p.quantity * price.close;
        let market_value_base = market_value * fx_rate;
        // Unrealized P/L uses historical base-currency cost, so FX movement remains visible.
        let unrealized = market_value_base - p.cost_basis_base;
        // Revaluing the cost at today's rate separates "the share rose" from "the dollar rose".
        let currency_gain = p.cost_basis * cost_fx_rate - p.cost_basis_base;

        securities_value_base += market_value_base;
        cost_basis_base += p.cost_basis_base;
        unrealized_total += unrealized;
        currency_total += currency_gain;

        positions.push(PositionValuation {
            security_id: security_id.clone(),
            currency: price.currency.clone(),
            cost_currency: p.cost_currency.clone(),
            quantity: p.quantity,
            price: price.close,
            fx_rate,
            cost_fx_rate,
            market_value,
            market_value_base,
            cost_basis: p.cost_basis,
            cost_basis_base: p.cost_basis_base,
            unrealized_pnl_base: unrealized,
            currency_gain_base: currency_gain,
            realized_pnl_base: p.realized_pnl_base,
            observed_quantity_step: p.observed_quantity_step(),
        });
    }

    let cash_base = holdings.cash_in_base(&base, date, rates)?;

    Ok(PortfolioValuation {
        date,
        base_currency: base,
        positions,
        securities_value_base,
        cash_base,
        total_value_base: securities_value_base + cash_base,
        cost_basis_base,
        unrealized_pnl_base: unrealized_total,
        currency_gain_base: currency_total,
        realized_pnl_base: holdings.realized_pnl_base,
        realized_currency_gain_base: holdings.realized_currency_gain_base,
        dividends_base: holdings.dividends_base,
        interest_base: holdings.interest_base,
        fees_base: holdings.fees_base,
        taxes_base: holdings.taxes_base,
    })
}

/// One position's change since the previous trading day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayChange {
    /// Previous close in the quote currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub previous_price: Decimal,
    /// Position value change in the base currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub change_base: Decimal,
    /// Relative change; computed here to keep money arithmetic out of JavaScript.
    #[serde(with = "rust_decimal::serde::str")]
    pub change: Decimal,
}

/// Per-position changes for the last trading day plus the total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DayChanges {
    /// Per-security changes; a missing key means the change is undefined, not zero.
    pub positions: BTreeMap<String, DayChange>,
    /// Sum over positions with a defined change.
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
}

/// Computes quote-price movement in base currency:
/// `(price - previous_close) * quantity * rate(date)`.
pub fn day_changes(
    holdings: &Holdings,
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<DayChanges> {
    let base = normalize_currency(base);
    let mut out = DayChanges::default();

    for (security_id, p) in &holdings.positions {
        if p.is_closed() {
            continue;
        }
        let (Some(price), Some(previous)) = (
            prices.price_as_of(security_id, date)?,
            prices.price_before(security_id, date)?,
        ) else {
            continue;
        };
        // A currency switch makes the price difference meaningless.
        if price.currency != previous.currency || previous.close.is_zero() {
            continue;
        }
        let rate =
            rates
                .rate_as_of(&price.currency, &base, date)?
                .ok_or_else(|| Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{}", price.currency, base),
                    date,
                })?;

        let diff = price.close - previous.close;
        let change_base = diff * p.quantity * rate;
        out.total_base += change_base;
        out.positions.insert(
            security_id.clone(),
            DayChange {
                previous_price: previous.close,
                change_base,
                change: diff / previous.close,
            },
        );
    }
    Ok(out)
}
