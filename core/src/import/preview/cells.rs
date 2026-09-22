//! The file's cells, after the user's per-cell overrides.
//!
//! Every stage downstream reads a row through here, so the preview and the commit see the same
//! input: a value corrected in the wizard is corrected once, before anything is parsed.

use super::RowOverride;
use crate::import::mapping::{ImportField, ImportMapping};
use crate::import::parse::{ImportProblem, ParsedCsv, ProblemCode, is_placeholder, parse_decimal};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// One source row ready to be read: the file's cells, and the user's hand edits beside them.
pub(super) struct RowInput {
    pub raw: BTreeMap<String, String>,
    /// Edits that have no column to live in. A delivery whose file states no price is corrected
    /// by *adding* the value, not by changing a cell, so an override of an unmapped field is
    /// carried on its own instead of being dropped.
    pub added: BTreeMap<ImportField, String>,
}

/// The file's rows with the overrides already folded in.
pub(super) fn rows_with_overrides(
    parsed: &ParsedCsv,
    mapping: &ImportMapping,
    overrides: &[RowOverride],
) -> Vec<RowInput> {
    parsed
        .rows
        .iter()
        .enumerate()
        .map(|(index, values)| {
            let mut raw: BTreeMap<String, String> = parsed
                .headers
                .iter()
                .cloned()
                .zip(values.iter().cloned())
                .collect();
            let mut added = BTreeMap::new();
            for o in overrides.iter().filter(|o| o.number == index + 1) {
                match mapping.column(o.field) {
                    Some(column) => {
                        raw.insert(column.to_string(), o.value.clone());
                    }
                    None => {
                        added.insert(o.field, o.value.clone());
                    }
                }
            }
            RowInput { raw, added }
        })
        .collect()
}

/// One row, read by field rather than by column name.
pub(super) struct Cells<'a> {
    pub raw: &'a BTreeMap<String, String>,
    pub mapping: &'a ImportMapping,
    pub decimal_separator: char,
    /// 1-based, as the user sees it in the wizard.
    pub number: usize,
    /// What the rule that produced this operation says instead of the row (ADR-0067). Empty
    /// for a row that became one operation, which is every row of most files.
    pub emitted: BTreeMap<ImportField, String>,
    /// Hand edits with no column of their own. They outrank a rule: the user is correcting the
    /// operation the rule produced, not asking for it to be produced differently.
    pub added: &'a BTreeMap<ImportField, String>,
}

impl Cells<'_> {
    pub fn get(&self, field: ImportField) -> Option<&str> {
        if let Some(value) = self.added.get(&field) {
            return Some(value.trim()).filter(|v| !v.is_empty());
        }
        match self.emitted.get(&field) {
            Some(value) => Some(value.trim()).filter(|v| !v.is_empty()),
            None => cell_of(self.raw, self.mapping, field),
        }
    }

    /// The column a problem points at. Empty when nothing is mapped to the field — the message
    /// still names the row, which is what the user navigates by.
    pub fn column(&self, field: ImportField) -> &str {
        self.mapping.column(field).unwrap_or_default()
    }

    /// An absent or placeholder cell is an absent number, not a broken one: a broker prints `-`
    /// for "no fee", and refusing the row over it would be refusing the file.
    pub fn decimal(&self, field: ImportField, problems: &mut Vec<ImportProblem>) -> Decimal {
        match self.get(field) {
            None => Decimal::ZERO,
            Some(value) if is_placeholder(value) => Decimal::ZERO,
            Some(value) => match parse_decimal(value, self.decimal_separator) {
                Some(d) => d,
                None => {
                    problems.push(ImportProblem::cell(
                        ProblemCode::NotANumber,
                        self.number,
                        self.column(field),
                        format!("not a number: {value:?}"),
                    ));
                    Decimal::ZERO
                }
            },
        }
    }
}

pub(super) fn cell_of<'a>(
    raw: &'a BTreeMap<String, String>,
    mapping: &ImportMapping,
    field: ImportField,
) -> Option<&'a str> {
    mapping
        .column(field)
        .and_then(|c| raw.get(c))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
}
