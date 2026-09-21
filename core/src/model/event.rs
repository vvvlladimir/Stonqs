use super::CorporateAction;
use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityEventKind {
    /// The user's own note on a date.
    Note,
    /// A dividend the provider reported, dated on its ex-date.
    Dividend,
    /// A split the provider reported, dated on its ex-date.
    Split,
}

impl SecurityEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityEventKind::Note => "NOTE",
            SecurityEventKind::Dividend => "DIVIDEND",
            SecurityEventKind::Split => "SPLIT",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "NOTE" => SecurityEventKind::Note,
            "DIVIDEND" => SecurityEventKind::Dividend,
            "SPLIT" => SecurityEventKind::Split,
            other => return Err(Error::Invalid(format!("unknown event kind {other:?}"))),
        })
    }
}

/// A dated fact about an instrument. It moves no money and no quantity: a reported dividend is
/// not income until a transaction says so, and a reported split is not applied until the user
/// records it as a [`CorporateAction`]. See ADR-0034.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub security_id: String,
    pub date: NaiveDate,
    pub kind: SecurityEventKind,
    /// Dividend per share, in `currency`.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub currency: Option<Currency>,
    /// A split reads like [`CorporateAction`]: 2:1 is `from = 1, to = 2`.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub ratio_from: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub ratio_to: Option<Decimal>,
    #[serde(default)]
    pub note: Option<String>,
    /// Provider id; `None` is the user's own event.
    #[serde(default)]
    pub source: Option<String>,
}

impl SecurityEvent {
    fn new(security_id: &str, date: NaiveDate, kind: SecurityEventKind) -> Self {
        SecurityEvent {
            id: super::new_id(),
            security_id: security_id.to_string(),
            date,
            kind,
            amount: None,
            currency: None,
            ratio_from: None,
            ratio_to: None,
            note: None,
            source: None,
        }
    }

    pub fn note(security_id: &str, date: NaiveDate, note: impl Into<String>) -> Self {
        SecurityEvent {
            note: Some(note.into()),
            ..Self::new(security_id, date, SecurityEventKind::Note)
        }
    }

    pub fn dividend(
        security_id: &str,
        date: NaiveDate,
        amount: Decimal,
        currency: &str,
        source: &str,
    ) -> Self {
        SecurityEvent {
            amount: Some(amount),
            currency: Some(normalize_currency(currency)),
            source: Some(source.to_string()),
            ..Self::new(security_id, date, SecurityEventKind::Dividend)
        }
    }

    pub fn split(security_id: &str, date: NaiveDate, from: Decimal, to: Decimal, source: &str) -> Self {
        SecurityEvent {
            ratio_from: Some(from),
            ratio_to: Some(to),
            source: Some(source.to_string()),
            ..Self::new(security_id, date, SecurityEventKind::Split)
        }
    }

    /// Whether a reported split is already applied: an action on the same instrument and ex-date.
    /// The ratio is not compared — a user who typed it differently still recorded that split.
    pub fn is_recorded_by(&self, action: &CorporateAction) -> bool {
        self.kind == SecurityEventKind::Split
            && action.security_id == self.security_id
            && action.date == self.date
    }

    pub fn validate(&self) -> Result<()> {
        match self.kind {
            SecurityEventKind::Note => {
                if self.note.as_deref().is_none_or(|n| n.trim().is_empty()) {
                    return Err(Error::Invalid("a note event needs text".into()));
                }
            }
            SecurityEventKind::Dividend => {
                if self.amount.is_none_or(|a| a <= Decimal::ZERO) || self.currency.is_none() {
                    return Err(Error::Invalid(
                        "a dividend event needs an amount and a currency".into(),
                    ));
                }
            }
            SecurityEventKind::Split => {
                let positive = |v: Option<Decimal>| v.is_some_and(|v| v > Decimal::ZERO);
                if !positive(self.ratio_from) || !positive(self.ratio_to) {
                    return Err(Error::Invalid(
                        "split ratio must be positive on both sides".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 6, d).unwrap()
    }

    #[test]
    fn a_split_is_recorded_by_an_action_on_its_ex_date() {
        let event = SecurityEvent::split("nvda", day(10), dec!(1), dec!(10), "yahoo");
        assert!(event.is_recorded_by(&CorporateAction::split("nvda", day(10), dec!(1), dec!(10))));
        assert!(!event.is_recorded_by(&CorporateAction::split("nvda", day(11), dec!(1), dec!(10))));
        assert!(!event.is_recorded_by(&CorporateAction::split("aapl", day(10), dec!(1), dec!(10))));
    }

    #[test]
    fn an_empty_note_is_refused() {
        assert!(SecurityEvent::note("s", day(1), "  ").validate().is_err());
        assert!(SecurityEvent::note("s", day(1), "AGM").validate().is_ok());
    }
}
