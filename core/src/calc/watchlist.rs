//! What a watched instrument's own closes say, in its own quote currency: the last price, the
//! day's and the period's move, the period's range, the high, what the provider reports it pays,
//! and the nearest trigger level. No position and no exchange rate are involved. See ADR-0035.

use super::{AlertStatus, peak_of};
use crate::market::PricePoint;
use crate::model::{AlertDirection, SecurityAlert, SecurityEvent, SecurityEventKind};
use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One instrument's price facts as of a day. Every field is `None` without a close on file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentMove {
    /// Currency of the latest close; every figure below is in it.
    pub currency: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    /// A quote days old is still the latest known one.
    pub price_date: Option<NaiveDate>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub previous_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub day_change: Option<Decimal>,
    /// The close the period's move is measured from: the one in force on `from`, or the first
    /// one after it for an instrument younger than the period.
    pub period_start: Option<NaiveDate>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub start_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub period_return: Option<Decimal>,
    /// Lowest and highest close from `period_start` through the latest one.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub low: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub high: Option<Decimal>,
    /// Where the price sits in that range: 0 at the low, 1 at the high; `None` for a flat range.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub range_position: Option<Decimal>,
    /// Highest stored close, and how far below it the price stands.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub ath_price: Option<Decimal>,
    pub ath_date: Option<NaiveDate>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub ath_distance: Option<Decimal>,
    /// Dividends per share the provider reported over the year ending at `to`, and that sum over
    /// the price. Only those in the price's currency count; `None` when there are none.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub dividend_year: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub dividend_yield: Option<Decimal>,
    pub dividend_last: Option<NaiveDate>,
}

/// Reads `prices` through `to`. A close in another currency than the latest one is left out: a
/// listing switch would otherwise divide euros by dollars.
pub fn instrument_move(
    prices: &BTreeMap<NaiveDate, PricePoint>,
    events: &[SecurityEvent],
    from: NaiveDate,
    to: NaiveDate,
) -> InstrumentMove {
    let Some((&price_date, last)) = prices.range(..=to).next_back() else {
        return InstrumentMove::default();
    };
    let currency = last.currency.clone();
    let price = last.close;
    let closes: Vec<(NaiveDate, Decimal)> = prices
        .range(..=to)
        .filter(|(_, p)| p.currency == currency)
        .map(|(date, p)| (*date, p.close))
        .collect();

    let previous_price = closes.len().checked_sub(2).map(|i| closes[i].1);
    let day_change = previous_price
        .filter(|p| !p.is_zero())
        .map(|p| price / p - Decimal::ONE);

    let start = closes
        .iter()
        .rev()
        .find(|(date, _)| *date <= from)
        .or_else(|| closes.iter().find(|(date, _)| *date > from))
        .copied();
    let period_return = start
        .filter(|(date, close)| *date < price_date && !close.is_zero())
        .map(|(_, close)| price / close - Decimal::ONE);

    let window = || {
        let since = start.map_or(price_date, |(date, _)| date);
        closes
            .iter()
            .filter(move |(date, _)| *date >= since)
            .map(|(_, c)| *c)
    };
    let low = window().min();
    let high = window().max();
    let range_position = match (low, high) {
        (Some(low), Some(high)) if high > low => Some((price - low) / (high - low)),
        _ => None,
    };

    let peak = peak_of(closes.iter().copied());

    let year_ago = to.checked_sub_months(Months::new(12)).unwrap_or(NaiveDate::MIN);
    let paid: Vec<&SecurityEvent> = events
        .iter()
        .filter(|e| {
            e.kind == SecurityEventKind::Dividend
                && e.source.is_some()
                && e.currency.as_deref() == Some(currency.as_str())
                && e.date <= to
        })
        .collect();
    let trailing: Vec<Decimal> = paid
        .iter()
        .filter(|e| e.date > year_ago)
        .filter_map(|e| e.amount)
        .collect();
    let dividend_year = (!trailing.is_empty()).then(|| trailing.iter().sum::<Decimal>());

    InstrumentMove {
        currency: Some(currency),
        price: Some(price),
        price_date: Some(price_date),
        previous_price,
        day_change,
        period_start: start.map(|(date, _)| date),
        start_price: start.map(|(_, close)| close),
        period_return,
        low,
        high,
        range_position,
        ath_price: peak.as_ref().map(|p| p.value),
        ath_date: peak.as_ref().map(|p| p.date),
        ath_distance: peak.and_then(|p| p.distance),
        dividend_yield: dividend_year
            .filter(|_| price > Decimal::ZERO)
            .map(|sum| sum / price),
        dividend_year,
        dividend_last: paid.iter().map(|e| e.date).max(),
    }
}

/// The trigger level the price is closest to, whichever side it is on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NearestLevel {
    pub alert_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub level: Decimal,
    pub currency: String,
    pub direction: AlertDirection,
    /// `level / price − 1`, as [`AlertStatus::distance`]: positive when the level is above.
    #[serde(with = "rust_decimal::serde::str")]
    pub distance: Decimal,
}

/// Picks the smallest absolute distance among price rules with a status; a date rule has none.
pub fn nearest_level<'a>(
    rules: impl IntoIterator<Item = (&'a SecurityAlert, &'a AlertStatus)>,
) -> Option<NearestLevel> {
    rules
        .into_iter()
        .filter_map(|(alert, status)| {
            Some(NearestLevel {
                alert_id: alert.id.clone(),
                level: alert.price?,
                currency: alert.currency.clone()?,
                direction: alert.direction,
                distance: status.distance?,
            })
        })
        .min_by_key(|level| level.distance.abs())
}
