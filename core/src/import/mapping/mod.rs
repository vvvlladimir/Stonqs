//! The user-editable mapping from a file's columns and values to the model's fields.
//!
//! Import is semi-automatic: everything detected here stays overridable, and nothing is silently
//! guessed for the user (`.claude/rules/import.md`). The dictionaries that drive detection are
//! data and live apart from the logic that reads them — `aliases` for headers, `keywords` for
//! operation wordings — because their order is load-bearing and their size is not interesting.

mod aliases;
mod field;
mod keywords;
mod normalize;
mod rules;
mod shape;

use super::securities::SecurityDraft;
use crate::model::TransactionKind;
use crate::money::{Currency, normalize_currency};
use serde::{Deserialize, Serialize};
use shape::shape_allows;
use std::collections::{BTreeMap, BTreeSet};

pub use field::ImportField;
pub(crate) use field::header_row_score;
pub use keywords::default_kind_aliases;
pub(crate) use keywords::kind_from_keywords;
pub use normalize::normalize_alias;
pub(crate) use normalize::normalize_header;
pub use rules::{Condition, Emit, ImportRule, Sign, Test};
pub(crate) use rules::{first_match, resolve};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmountSign {
    /// Amount signs may reverse reversible cash operation kinds.
    Signed,

    /// Operation kinds provide direction; amount signs are ignored.
    Unsigned,
}

/// Whether the file's amount column already has the row's charges taken out of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmountBasis {
    /// The amount is the trade's own value; commission and tax sit beside it.
    Gross,

    /// The amount is what the account was actually debited or credited, charges included —
    /// so the stored gross has to be restored from it.
    Net,
}

/// User-editable mapping from file values and columns to model fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportMapping {
    pub account_id: Option<String>,

    pub columns: BTreeMap<ImportField, String>,

    pub kind_aliases: BTreeMap<String, TransactionKind>,

    /// Operation values the user decided to skip. A broker prints lines that are not
    /// operations at all ("Monthly statement", "Name change"), and refusing to import the
    /// file because of them is not an answer.
    #[serde(default)]
    pub ignored_kinds: BTreeSet<String>,

    pub symbol_aliases: BTreeMap<String, String>,

    pub account_aliases: BTreeMap<String, String>,

    pub default_currency: Option<Currency>,

    #[serde(default)]
    pub amount_sign: Option<AmountSign>,

    /// Whether `amount` is gross or net of the row's own charges. Absent is decided from the
    /// file: a broker that prints both the total and the commission says which it meant.
    #[serde(default)]
    pub amount_basis: Option<AmountBasis>,

    /// What a row becomes when its wording is not the whole answer: a condition and the
    /// operations it produces. Read before `kind_aliases`, which stays the common case
    /// (ADR-0067).
    #[serde(default)]
    pub rules: Vec<ImportRule>,

    #[serde(default)]
    pub new_securities: BTreeMap<String, SecurityDraft>,
}

impl ImportMapping {
    /// Detects a non-conflicting initial mapping from the headers alone.
    pub fn detect(headers: &[String]) -> Self {
        Self::detect_with_values(headers, &[])
    }

    /// Detects the mapping from headers *and* values. Values arbitrate a weak header match
    /// and find a column no alias names, which is what keeps the detection language-neutral.
    pub fn detect_with_values(headers: &[String], rows: &[Vec<String>]) -> Self {
        let column = |index: usize| -> Vec<&str> {
            rows.iter()
                .filter_map(|r| r.get(index))
                .map(String::as_str)
                .collect()
        };

        let mut candidates: Vec<(u32, ImportField, usize)> = Vec::new();
        for field in ImportField::ALL {
            let shape = field.value_shape();
            for (index, header) in headers.iter().enumerate() {
                let Some(found) = field.header_match(header) else {
                    continue;
                };
                let values = column(index);
                // A column that is empty everywhere is not a mapping: Investimental leaves
                // "Settlement Date" blank and keeps the real one in another column.
                if !rows.is_empty() && values.iter().all(|v| v.trim().is_empty()) {
                    continue;
                }
                if !shape_allows(shape, found.tier, &values) {
                    continue;
                }
                candidates.push((found.score, *field, index));
            }
        }

        // The best *surviving* claim on a column also settles what the column is *not*: a
        // lower-scoring field never takes a column another field names better ("Asset type" is
        // a kind, so it is not a symbol), even when that other field is already mapped
        // elsewhere. A field the values vetoed is not such a claim — "Transaction ID" is an
        // external id when its values are unique and a pairing key when they repeat, and
        // whichever it is must not block the other.
        let mut best_on_column = vec![0u32; headers.len()];
        for (score, _, index) in &candidates {
            best_on_column[*index] = best_on_column[*index].max(*score);
        }
        candidates.retain(|(score, _, index)| *score >= best_on_column[*index]);

        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

        let mut columns = BTreeMap::new();
        let mut taken = vec![false; headers.len()];
        for (_, field, index) in candidates {
            if columns.contains_key(&field) || taken[index] {
                continue;
            }
            taken[index] = true;
            columns.insert(field, headers[index].clone());
        }

        // A column whose header no language of ours names is still recognisable by its
        // values: Parqet calls its ISIN column "identifier".
        for field in [ImportField::Isin, ImportField::Date] {
            if columns.contains_key(&field) {
                continue;
            }
            let shape = field.value_shape();
            let found = (0..headers.len())
                .filter(|i| !taken[*i])
                .find(|i| shape.fit(&column(*i)).is_some_and(|fit| fit >= 0.8));
            if let Some(index) = found {
                taken[index] = true;
                columns.insert(field, headers[index].clone());
            }
        }

        let kind_values: Vec<&str> = columns
            .get(&ImportField::Kind)
            .and_then(|name| headers.iter().position(|h| h == name))
            .map(column)
            .unwrap_or_default();

        ImportMapping {
            columns,
            kind_aliases: default_kind_aliases(),
            ..ImportMapping::default()
        }
        .with_detected_kinds(kind_values.iter().copied())
    }

    /// Reads the file's own wording for an operation and proposes a kind for every value
    /// nobody has answered yet. A broker prints wordings no list can enumerate ("Sell 3 @
    /// 139.74 USD"), so a saved layout has to keep learning from the file it is applied to.
    /// Values already aliased or already skipped are left exactly as they are.
    pub fn with_detected_kinds<'a>(mut self, values: impl Iterator<Item = &'a str>) -> Self {
        for value in values {
            let key = normalize_alias(value);
            if key.is_empty() || self.kind_aliases.contains_key(&key) || self.ignored_kinds.contains(&key) {
                continue;
            }
            if let Some(kind) = kind_from_keywords(&key) {
                self.kind_aliases.insert(key, kind);
            }
        }
        self
    }

    pub fn column(&self, field: ImportField) -> Option<&str> {
        self.columns.get(&field).map(String::as_str)
    }

    pub fn set_column(&mut self, field: ImportField, column: Option<&str>) {
        match column {
            Some(c) => {
                self.columns.insert(field, c.to_string());
            }
            None => {
                self.columns.remove(&field);
            }
        }
    }

    pub fn with_column(mut self, field: ImportField, column: &str) -> Self {
        self.set_column(field, Some(column));
        self
    }

    pub fn with_kind_alias(mut self, value: &str, kind: TransactionKind) -> Self {
        self.kind_aliases.insert(normalize_alias(value), kind);
        self
    }

    pub fn with_new_security(mut self, value: &str, draft: SecurityDraft) -> Self {
        self.new_securities.insert(normalize_alias(value), draft);
        self
    }

    pub fn new_security_for(&self, value: &str) -> Option<&SecurityDraft> {
        self.new_securities
            .get(&normalize_alias(value))
            .or_else(|| self.new_securities.get(&normalize_alias(self.symbol_of(value))))
    }

    pub fn with_symbol_alias(mut self, from: &str, to: &str) -> Self {
        self.symbol_aliases.insert(normalize_alias(from), to.to_string());
        self
    }

    pub fn with_account(mut self, account_id: &str) -> Self {
        self.account_id = Some(account_id.to_string());
        self
    }

    pub fn with_amount_sign(mut self, sign: AmountSign) -> Self {
        self.amount_sign = Some(sign);
        self
    }

    pub fn with_amount_basis(mut self, basis: AmountBasis) -> Self {
        self.amount_basis = Some(basis);
        self
    }

    pub fn with_rule(mut self, rule: ImportRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn with_default_currency(mut self, currency: &str) -> Self {
        self.default_currency = Some(normalize_currency(currency));
        self
    }

    pub fn with_ignored_kind(mut self, value: &str) -> Self {
        self.ignored_kinds.insert(normalize_alias(value));
        self
    }

    /// Whether rows carrying this operation value are to be left out of the import.
    pub fn is_ignored(&self, value: &str) -> bool {
        self.ignored_kinds.contains(&normalize_alias(value))
    }

    pub fn kind_of(&self, value: &str) -> Option<TransactionKind> {
        self.kind_aliases.get(&normalize_alias(value)).copied()
    }

    pub fn symbol_of<'a>(&'a self, value: &'a str) -> &'a str {
        self.symbol_aliases
            .get(&normalize_alias(value))
            .map(String::as_str)
            .unwrap_or(value)
    }

    pub fn missing_required(&self) -> Vec<ImportField> {
        ImportField::ALL
            .iter()
            .copied()
            .filter(|f| f.is_required() && !self.columns.contains_key(f))
            .collect()
    }
}

#[cfg(test)]
mod tests;
