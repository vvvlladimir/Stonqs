use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertKind {
    /// A trigger level; every close that lands on the other side of it is a crossing.
    Price,
    /// A day; reaching it is logged once.
    DateReached,
}

impl AlertKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AlertKind::Price => "PRICE",
            AlertKind::DateReached => "DATE_REACHED",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "PRICE" => AlertKind::Price,
            "DATE_REACHED" => AlertKind::DateReached,
            other => return Err(Error::Invalid(format!("unknown alert kind {other:?}"))),
        })
    }
}

/// Where a close stands against the level. A close exactly at the level counts as above, so
/// rising *to* the level is a crossing and so is falling back under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertSide {
    Above,
    Below,
}

impl AlertSide {
    pub fn as_str(self) -> &'static str {
        match self {
            AlertSide::Above => "ABOVE",
            AlertSide::Below => "BELOW",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "ABOVE" => AlertSide::Above,
            "BELOW" => AlertSide::Below,
            other => return Err(Error::Invalid(format!("unknown alert side {other:?}"))),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CrossingDirection {
    Up,
    Down,
    /// A date rule's day arrived.
    Reached,
}

impl CrossingDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            CrossingDirection::Up => "UP",
            CrossingDirection::Down => "DOWN",
            CrossingDirection::Reached => "REACHED",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "UP" => CrossingDirection::Up,
            "DOWN" => CrossingDirection::Down,
            "REACHED" => CrossingDirection::Reached,
            other => return Err(Error::Invalid(format!("unknown crossing direction {other:?}"))),
        })
    }
}

/// Which crossings of its level a price trigger logs and announces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertDirection {
    /// The price rises to or above the level.
    Up,
    /// The price falls under the level.
    Down,
    #[default]
    Both,
}

impl AlertDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            AlertDirection::Up => "UP",
            AlertDirection::Down => "DOWN",
            AlertDirection::Both => "BOTH",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "UP" => AlertDirection::Up,
            "DOWN" => AlertDirection::Down,
            "BOTH" => AlertDirection::Both,
            other => return Err(Error::Invalid(format!("unknown alert direction {other:?}"))),
        })
    }

    /// Whether a move onto `side` is a crossing this trigger logs.
    pub fn accepts(self, side: AlertSide) -> bool {
        match self {
            AlertDirection::Up => side == AlertSide::Above,
            AlertDirection::Down => side == AlertSide::Below,
            AlertDirection::Both => true,
        }
    }
}

/// A trigger the user set on an instrument: a price level or a date. See ADR-0034.
///
/// `side` and `checked_through` are the check's bookmark: the side of the last close it read and
/// that close's date. A crossing is a change of side, logged as an [`AlertCrossing`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub id: String,
    pub security_id: String,
    pub kind: AlertKind,
    /// The level of a price trigger, in `currency`.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    /// The currency the level is written in; a quote in another one is converted at its day's rate.
    #[serde(default)]
    pub currency: Option<Currency>,
    /// The day of a date rule.
    #[serde(default)]
    pub date: Option<NaiveDate>,
    #[serde(default)]
    pub note: Option<String>,
    /// The close on or before this day is where the trigger starts from, never a crossing itself.
    pub created_on: NaiveDate,
    #[serde(default)]
    pub side: Option<AlertSide>,
    #[serde(default)]
    pub checked_through: Option<NaiveDate>,
    /// Which crossings a price trigger logs; the bookmark follows every change of side regardless.
    #[serde(default)]
    pub direction: AlertDirection,
}

impl SecurityAlert {
    fn new(security_id: &str, kind: AlertKind, created_on: NaiveDate) -> Self {
        SecurityAlert {
            id: super::new_id(),
            security_id: security_id.to_string(),
            kind,
            price: None,
            currency: None,
            date: None,
            note: None,
            created_on,
            side: None,
            checked_through: None,
            direction: AlertDirection::Both,
        }
    }

    pub fn price(security_id: &str, level: Decimal, currency: &str, created_on: NaiveDate) -> Self {
        SecurityAlert {
            price: Some(level),
            currency: Some(normalize_currency(currency)),
            ..Self::new(security_id, AlertKind::Price, created_on)
        }
    }

    pub fn date_reached(security_id: &str, date: NaiveDate, created_on: NaiveDate) -> Self {
        SecurityAlert {
            date: Some(date),
            ..Self::new(security_id, AlertKind::DateReached, created_on)
        }
    }

    pub fn with_direction(mut self, direction: AlertDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Whether two rules watch the same thing; a different note or bookmark does not count.
    pub fn same_trigger(&self, other: &SecurityAlert) -> bool {
        self.security_id == other.security_id
            && self.kind == other.kind
            && self.price == other.price
            && self.currency == other.currency
            && self.date == other.date
            && self.direction == other.direction
    }

    pub fn validate(&self) -> Result<()> {
        match self.kind {
            AlertKind::Price => {
                if self.price.is_none_or(|p| p <= Decimal::ZERO) {
                    return Err(Error::Invalid("a price alert needs a level above zero".into()));
                }
                if self.currency.as_deref().is_none_or(|c| c.trim().is_empty()) {
                    return Err(Error::Invalid(
                        "a price alert needs the currency of its level".into(),
                    ));
                }
            }
            AlertKind::DateReached => {
                if self.date.is_none() {
                    return Err(Error::Invalid("a date alert needs a date".into()));
                }
            }
        }
        Ok(())
    }
}

/// One line of an alert's log: the close that crossed the level, or the day a date rule reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertCrossing {
    /// Assigned by storage; zero until written.
    pub id: i64,
    pub alert_id: String,
    pub date: NaiveDate,
    pub direction: CrossingDirection,
    /// The level as it was when crossed.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub level: Option<Decimal>,
    /// The close that crossed, in `currency`.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<Currency>,
    pub seen: bool,
    pub notified: bool,
}

impl AlertCrossing {
    pub fn new(
        alert: &SecurityAlert,
        date: NaiveDate,
        direction: CrossingDirection,
        price: Option<Decimal>,
    ) -> Self {
        AlertCrossing {
            id: 0,
            alert_id: alert.id.clone(),
            date,
            direction,
            level: alert.price,
            price,
            currency: alert.currency.clone(),
            seen: false,
            notified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 6, 3).unwrap()
    }

    #[test]
    fn a_rule_is_refused_without_what_it_compares_against() {
        assert!(
            SecurityAlert::price("s", dec!(10), "usd", day())
                .validate()
                .is_ok()
        );
        assert!(
            SecurityAlert::price("s", dec!(0), "USD", day())
                .validate()
                .is_err()
        );
        assert!(SecurityAlert::date_reached("s", day(), day()).validate().is_ok());

        let mut no_date = SecurityAlert::date_reached("s", day(), day());
        no_date.date = None;
        assert!(no_date.validate().is_err());
    }

    #[test]
    fn a_new_note_is_the_same_trigger_and_a_new_level_is_not() {
        let rule = SecurityAlert::price("s", dec!(10), "USD", day());
        assert_eq!(rule.currency.as_deref(), Some("USD"));
        assert!(rule.same_trigger(&rule.clone().with_note("trim")));
        assert!(!rule.same_trigger(&SecurityAlert::price("s", dec!(11), "USD", day())));
    }
}
