//! Price triggers and date rules: which closes crossed the level, and where the price stands now.
//! See ADR-0034.

use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::PricePoint;
use crate::model::{AlertCrossing, AlertKind, AlertSide, CrossingDirection, SecurityAlert};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where the price stands against a trigger today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertStatus {
    /// Latest close in the level's currency; `None` for a date rule.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    /// Day of that close — a quote days old is still the latest known one.
    pub price_date: Option<NaiveDate>,
    pub side: Option<AlertSide>,
    /// How far the price has to move to reach the level, as a fraction of the price:
    /// `level / price − 1`, positive when the level is above.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub distance: Option<Decimal>,
}

/// What a check found: the new bookmark and the crossings since the old one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertCheck {
    pub side: Option<AlertSide>,
    pub checked_through: Option<NaiveDate>,
    pub crossings: Vec<AlertCrossing>,
}

impl AlertCheck {
    /// Whether anything has to be written back.
    pub fn changes(&self, alert: &SecurityAlert) -> bool {
        !self.crossings.is_empty() || self.side != alert.side || self.checked_through != alert.checked_through
    }
}

fn side_of(close: Decimal, level: Decimal) -> AlertSide {
    if close >= level {
        AlertSide::Above
    } else {
        AlertSide::Below
    }
}

/// A close in the level's currency, converted at the rate of the quote's own day.
fn in_currency(
    point: &PricePoint,
    currency: &str,
    date: NaiveDate,
    rates: &impl RateLookup,
) -> Result<Decimal> {
    if point.currency == currency {
        return Ok(point.close);
    }
    let rate = rates
        .rate_as_of(&point.currency, currency, date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{}/{currency}", point.currency),
            date,
        })?;
    Ok(point.close * rate)
}

/// Reads the closes from the alert's bookmark through `today` and logs every change of side.
///
/// The first close ever read — the one on or before `created_on`, else the first one after —
/// only sets the side: the trigger starts from where the price was. The check restarts at the
/// last close still on file up to `checked_through`, so a rewritten last quote is read again
/// and a crossing it now makes is not missed. No quote at all changes nothing.
pub fn check_alert(
    alert: &SecurityAlert,
    prices: &BTreeMap<NaiveDate, PricePoint>,
    rates: &impl RateLookup,
    today: NaiveDate,
) -> Result<AlertCheck> {
    alert.validate()?;
    let mut check = AlertCheck {
        side: alert.side,
        checked_through: alert.checked_through,
        crossings: Vec::new(),
    };

    match alert.kind {
        AlertKind::DateReached => {
            let date = alert.date.expect("validated: a date rule has a date");
            if today >= date && alert.checked_through != Some(date) {
                check
                    .crossings
                    .push(AlertCrossing::new(alert, date, CrossingDirection::Reached, None));
                check.checked_through = Some(date);
            }
        }
        AlertKind::Price => {
            let level = alert.price.expect("validated: a price rule has a level");
            let currency = alert
                .currency
                .as_deref()
                .expect("validated: a price rule has a currency");
            let anchor = alert.checked_through.unwrap_or(alert.created_on).min(today);
            let from = prices
                .range(..=anchor)
                .next_back()
                .map_or(anchor, |(date, _)| *date);
            if from > today {
                return Ok(check);
            }
            for (&date, point) in prices.range(from..=today) {
                let close = in_currency(point, currency, date, rates)?;
                let now = side_of(close, level);
                if let Some(before) = check.side
                    && before != now
                    && alert.direction.accepts(now)
                {
                    let direction = match now {
                        AlertSide::Above => CrossingDirection::Up,
                        AlertSide::Below => CrossingDirection::Down,
                    };
                    check
                        .crossings
                        .push(AlertCrossing::new(alert, date, direction, Some(close)));
                }
                check.side = Some(now);
                check.checked_through = Some(date);
            }
        }
    }
    Ok(check)
}

/// The latest close against the level. A price trigger without a quote, or without the rate its
/// quote needs, is `MissingMarketData`; a date rule has nothing to report.
pub fn alert_status(
    alert: &SecurityAlert,
    prices: &BTreeMap<NaiveDate, PricePoint>,
    rates: &impl RateLookup,
    today: NaiveDate,
) -> Result<AlertStatus> {
    alert.validate()?;
    if alert.kind == AlertKind::DateReached {
        return Ok(AlertStatus {
            price: None,
            price_date: None,
            side: None,
            distance: None,
        });
    }
    let level = alert.price.expect("validated: a price rule has a level");
    let currency = alert
        .currency
        .as_deref()
        .expect("validated: a price rule has a currency");
    let (&date, point) = prices
        .range(..=today)
        .next_back()
        .ok_or_else(|| Error::MissingMarketData {
            kind: "price",
            key: alert.security_id.clone(),
            date: today,
        })?;
    let close = in_currency(point, currency, date, rates)?;
    Ok(AlertStatus {
        price: Some(close),
        price_date: Some(date),
        side: Some(side_of(close, level)),
        distance: (!close.is_zero()).then(|| (level / close - Decimal::ONE).round_dp(6)),
    })
}

/// A close one percent past the level on the side the price is not on — what a simulated quote
/// needs to be to cross it. For a debug build's alert simulator only.
pub fn crossing_close(alert: &SecurityAlert) -> Option<Decimal> {
    let level = alert.price?;
    let factor = match alert.side {
        Some(AlertSide::Above) => Decimal::new(99, 2),
        _ => Decimal::new(101, 2),
    };
    Some((level * factor).round_dp(4))
}
