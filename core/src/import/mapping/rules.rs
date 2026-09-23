//! Rules: what a file row becomes, when one wording is not the whole answer.
//!
//! The kind dictionary answers "this word means this operation", which is most of every broker
//! file and none of the rest: a reinvested dividend is an income *and* a purchase, a wording
//! that spans both directions is told apart by the sign of a different column, and a line that
//! is not an operation at all has to be dropped by something other than its name.
//!
//! A rule is deliberately not a language. It matches on the *mapped fields* — never on raw
//! column names, so it survives a change of layout — with a closed set of tests joined by AND,
//! and it emits a fixed list of operations whose fields are either constants or references to
//! the row's own values. No arithmetic, no loops, no variables: a rule that cannot be read aloud
//! in one sentence is a rule nobody can review (ADR-0067).

use super::ImportField;
use crate::import::parse::parse_decimal;
use crate::model::TransactionKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a single field has to look like.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Test {
    /// The whole value, compared the way every other wording is: case, spaces and punctuation
    /// folded away.
    Equals(String),
    Contains(String),
    /// The sign of a number, for a wording that spans both directions.
    Sign(Sign),
    /// The field has a value at all, or has none.
    Present,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sign {
    Positive,
    Negative,
    Zero,
}

/// One field and the test it has to pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub field: ImportField,
    #[serde(flatten)]
    pub test: Test,
}

/// One operation the rule produces. `set` replaces what the row says, by field: a constant, or
/// `{field}` naming another of the row's own values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emit {
    pub kind: TransactionKind,
    #[serde(default)]
    pub set: BTreeMap<ImportField, String>,
}

impl Emit {
    pub fn of(kind: TransactionKind) -> Self {
        Emit {
            kind,
            set: BTreeMap::new(),
        }
    }

    pub fn with(mut self, field: ImportField, value: &str) -> Self {
        self.set.insert(field, value.to_string());
        self
    }
}

/// A match and what it produces. An empty `emit` is a row left out of the import — a broker
/// prints lines that are not operations, and some of them are only recognisable by a condition
/// rather than by a wording on the skip list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRule {
    #[serde(default)]
    pub when: Vec<Condition>,
    #[serde(default)]
    pub emit: Vec<Emit>,
    /// Whether the operations produced here are the legs of one internal move. They then share
    /// a link id, so `calc` reads them as money staying inside the portfolio.
    #[serde(default)]
    pub link: bool,
}

impl ImportRule {
    pub fn when(field: ImportField, test: Test) -> Self {
        ImportRule {
            when: vec![Condition { field, test }],
            ..ImportRule::default()
        }
    }

    pub fn and(mut self, field: ImportField, test: Test) -> Self {
        self.when.push(Condition { field, test });
        self
    }

    pub fn emit(mut self, emit: Emit) -> Self {
        self.emit.push(emit);
        self
    }

    pub fn linked(mut self) -> Self {
        self.link = true;
        self
    }

    /// Whether every condition holds. A rule with no conditions matches nothing: it would
    /// otherwise swallow the whole file, which is never what an empty rule was meant to say.
    pub fn matches(&self, row: &dyn Fn(ImportField) -> Option<String>, decimal_separator: char) -> bool {
        !self.when.is_empty()
            && self
                .when
                .iter()
                .all(|c| holds(&c.test, row(c.field).as_deref(), decimal_separator))
    }
}

fn holds(test: &Test, value: Option<&str>, decimal_separator: char) -> bool {
    match test {
        Test::Present => value.is_some(),
        Test::Empty => value.is_none(),
        Test::Equals(wanted) => {
            value.is_some_and(|v| super::normalize_alias(v) == super::normalize_alias(wanted))
        }
        Test::Contains(part) => {
            value.is_some_and(|v| super::normalize_alias(v).contains(&super::normalize_alias(part)))
        }
        Test::Sign(wanted) => {
            let number = value
                .and_then(|v| parse_decimal(v, decimal_separator))
                .unwrap_or_default();
            match wanted {
                Sign::Positive => number.is_sign_positive() && !number.is_zero(),
                Sign::Negative => number.is_sign_negative() && !number.is_zero(),
                Sign::Zero => number.is_zero(),
            }
        }
    }
}

/// The first rule the row answers. Order is the rule list's own, like every other dictionary
/// here: the first hit wins, so a narrow rule is written above a broad one.
pub(crate) fn first_match<'a>(
    rules: &'a [ImportRule],
    row: &dyn Fn(ImportField) -> Option<String>,
    decimal_separator: char,
) -> Option<&'a ImportRule> {
    rules.iter().find(|rule| rule.matches(row, decimal_separator))
}

/// Resolves what an emitted operation says in one field: a constant, or `{field}` naming a
/// value of the row it came from. An unknown name resolves to nothing rather than to its own
/// text — a typo must not become an amount.
pub(crate) fn resolve(template: &str, row: &dyn Fn(ImportField) -> Option<String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|i| start + i) else {
            break;
        };
        let name = &rest[start + 1..end];
        if let Some(field) = field_named(name) {
            out.push_str(&row(field).unwrap_or_default());
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// A field by the name a rule writes it under: its wire name, lower case (`amount`,
/// `external_id`).
fn field_named(name: &str) -> Option<ImportField> {
    let wanted = name.trim().to_ascii_uppercase();
    ImportField::ALL
        .iter()
        .copied()
        .find(|f| format!("{f:?}").to_ascii_uppercase() == wanted.replace('_', ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_of<'a>(values: &'a [(ImportField, &'a str)]) -> impl Fn(ImportField) -> Option<String> + 'a {
        move |field| {
            values
                .iter()
                .find(|(f, _)| *f == field)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn every_condition_has_to_hold() {
        let row = row_of(&[
            (ImportField::Kind, "Balance Conversion"),
            (ImportField::Amount, "-40"),
        ]);
        let rule = ImportRule::when(ImportField::Kind, Test::Contains("conversion".into()))
            .and(ImportField::Amount, Test::Sign(Sign::Negative))
            .emit(Emit::of(TransactionKind::TransferOut));

        assert!(rule.matches(&row, '.'));

        let positive = row_of(&[
            (ImportField::Kind, "Balance Conversion"),
            (ImportField::Amount, "40"),
        ]);
        assert!(!rule.matches(&positive, '.'));
    }

    #[test]
    fn a_rule_with_no_conditions_matches_nothing() {
        let row = row_of(&[(ImportField::Kind, "anything")]);
        assert!(!ImportRule::default().matches(&row, '.'));
    }

    #[test]
    fn a_value_is_a_constant_or_one_of_the_rows_own_fields() {
        let row = row_of(&[(ImportField::Amount, "40.00"), (ImportField::ExternalId, "TR-1")]);
        assert_eq!(resolve("{amount}", &row), "40.00");
        assert_eq!(resolve("{external_id}-a", &row), "TR-1-a");
        assert_eq!(resolve("0", &row), "0");
        // A name no field answers to is not text to be passed through.
        assert_eq!(resolve("{nonsense}", &row), "");
        // A field the row does not have reads as empty, not as the placeholder.
        assert_eq!(resolve("{note}", &row), "");
    }
}
