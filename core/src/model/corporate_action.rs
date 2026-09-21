use crate::error::{Error, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CorporateActionKind {
    /// Split or reverse split. Position value is unchanged; only quantity and per-share price move.
    Split,
}

/// A quantity-changing event with no trade behind it (kept out of `Transaction`
/// since it moves no money). Adjusts lot quantity only — quotes already arrive split-adjusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorporateAction {
    pub id: String,
    pub security_id: String,
    /// Ex-date: new quantity applies from this date on.
    pub date: NaiveDate,
    pub kind: CorporateActionKind,
    /// 2:1 split ("one old share becomes two") is `from = 1, to = 2`;
    /// 1:10 reverse split is `from = 10, to = 1`.
    #[serde(with = "rust_decimal::serde::str")]
    pub ratio_from: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub ratio_to: Decimal,
    pub note: Option<String>,
}

impl CorporateAction {
    pub fn split(security_id: &str, date: NaiveDate, from: Decimal, to: Decimal) -> Self {
        CorporateAction {
            id: super::new_id(),
            security_id: security_id.to_string(),
            date,
            kind: CorporateActionKind::Split,
            ratio_from: from,
            ratio_to: to,
            note: None,
        }
    }

    /// Quantity multiplier (`2` for a 2:1 split); per-share cost basis divides by the same factor.
    pub fn quantity_factor(&self) -> Result<Decimal> {
        if self.ratio_from.is_zero() {
            return Err(Error::Invalid(format!("split {} has zero ratio_from", self.id)));
        }
        Ok(self.ratio_to / self.ratio_from)
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.ratio_from <= Decimal::ZERO || self.ratio_to <= Decimal::ZERO {
            return Err(Error::Invalid(
                "split ratio must be positive on both sides".into(),
            ));
        }
        Ok(())
    }
}

impl CorporateActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CorporateActionKind::Split => "SPLIT",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "SPLIT" => Ok(CorporateActionKind::Split),
            other => Err(Error::Invalid(format!("unknown corporate action {other:?}"))),
        }
    }
}
