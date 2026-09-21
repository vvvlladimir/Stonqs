use crate::error::{Error, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// How an attribute's value is read; the value is stored as text in every case.
///
/// There is deliberately no percent kind: it would have to decide between `0.07` and `7`
/// for every reader, and nothing computes with attributes yet. A rate is a [`Number`]
/// carrying `%` as its unit.
///
/// [`Number`]: AttributeKind::Number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttributeKind {
    Text,
    Number,
    Date,
}

/// An attribute the user invented: TER, country of risk, replication method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAttributeDef {
    pub id: String,
    /// User data, seeded by nobody and never translated.
    pub name: String,
    pub kind: AttributeKind,
    /// Shown after the value (`%`, `bp`, `years`); never parsed.
    pub unit: Option<String>,
    /// Order in the editor; equal positions are ordered by name.
    pub position: i64,
}

impl SecurityAttributeDef {
    pub fn new(name: impl Into<String>, kind: AttributeKind) -> Self {
        SecurityAttributeDef {
            id: super::new_id(),
            name: name.into(),
            kind,
            unit: None,
            position: 0,
        }
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("an attribute needs a name".into()));
        }
        Ok(())
    }
}

impl AttributeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AttributeKind::Text => "TEXT",
            AttributeKind::Number => "NUMBER",
            AttributeKind::Date => "DATE",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "TEXT" => AttributeKind::Text,
            "NUMBER" => AttributeKind::Number,
            "DATE" => AttributeKind::Date,
            other => return Err(Error::Invalid(format!("unknown attribute kind {other:?}"))),
        })
    }

    /// Normalizes what the user typed, so a value written today reads back the same tomorrow:
    /// a number loses its trailing zeros, a date keeps the sortable `YYYY-MM-DD` form.
    pub fn normalize(self, value: &str) -> Result<String> {
        let value = value.trim();
        Ok(match self {
            AttributeKind::Text => value.to_string(),
            AttributeKind::Number => Decimal::from_str(value)
                .map_err(|e| Error::Invalid(format!("{value:?} is not a number: {e}")))?
                .normalize()
                .to_string(),
            AttributeKind::Date => NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|e| Error::Invalid(format!("{value:?} is not a date: {e}")))?
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AttributeKind, SecurityAttributeDef};

    #[test]
    fn a_value_is_normalized_by_its_kind() {
        // 0.0700 and 0.07 are the same TER; storing both would make two spellings of one fact.
        assert_eq!(AttributeKind::Number.normalize(" 0.0700 ").unwrap(), "0.07");
        assert_eq!(AttributeKind::Date.normalize("2024-06-03").unwrap(), "2024-06-03");
        assert_eq!(AttributeKind::Text.normalize("  Ireland  ").unwrap(), "Ireland");

        assert!(AttributeKind::Number.normalize("cheap").is_err());
        assert!(AttributeKind::Date.normalize("03.06.2024").is_err());
    }

    #[test]
    fn an_attribute_needs_a_name() {
        assert!(
            SecurityAttributeDef::new("TER", AttributeKind::Number)
                .validate()
                .is_ok()
        );
        assert!(
            SecurityAttributeDef::new("  ", AttributeKind::Text)
                .validate()
                .is_err()
        );
    }
}
