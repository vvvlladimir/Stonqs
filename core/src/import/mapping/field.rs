//! The fields a column can be mapped to, and how strongly a header claims one.
//!
//! Headers lie and values do not, so a claim is scored by tier — exact, whole word, bare
//! substring — and a weak one can still be vetoed by what the column actually contains
//! (`.claude/rules/import.md`). The alias dictionaries themselves live in `aliases`.

use super::normalize::{header_words, joined_words, normalize_header};
use super::shape::{HeaderMatch, MatchTier, ValueShape};
use serde::{Deserialize, Serialize};

/// A transaction field that can be mapped from a CSV column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImportField {
    Date,

    Kind,
    Symbol,
    Isin,

    Name,
    Quantity,
    Price,

    Amount,
    Fee,
    Tax,
    Currency,

    FxRate,

    Account,

    LinkId,
    Note,
}

impl ImportField {
    /// All fields exposed by the column-mapping UI.
    pub const ALL: &'static [ImportField] = &[
        ImportField::Date,
        ImportField::Kind,
        ImportField::Symbol,
        ImportField::Isin,
        ImportField::Name,
        ImportField::Quantity,
        ImportField::Price,
        ImportField::Amount,
        ImportField::Fee,
        ImportField::Tax,
        ImportField::Currency,
        ImportField::FxRate,
        ImportField::Account,
        ImportField::LinkId,
        ImportField::Note,
    ];

    pub fn is_required(self) -> bool {
        matches!(self, ImportField::Date | ImportField::Kind)
    }

    /// Scores exact aliases above a whole word, and a whole word above a bare substring —
    /// otherwise the Swedish "Instrumentvaluta" reads as an instrument.
    pub(crate) fn header_score(self, header: &str) -> Option<u32> {
        self.header_match(header).map(|m| m.score)
    }

    pub(crate) fn header_match(self, header: &str) -> Option<HeaderMatch> {
        let compact = normalize_header(header);
        let words = header_words(header);

        let mut best: Option<HeaderMatch> = None;
        for (index, alias) in self.header_aliases().iter().enumerate() {
            let alias = normalize_header(alias);
            // A word buried in a long header is a weak claim: "Price per Share in Account
            // Currency" is not the account column. Demanding a third of the words keeps
            // "Transaction Time (CET)" a date and drops that one.
            let spanned = joined_words(&words, &alias);
            let tier = if compact == alias {
                MatchTier::Exact
            } else if spanned.is_some_and(|n| n * 3 >= words.len()) {
                MatchTier::Word
            } else if spanned.is_some() || (alias.chars().count() >= 4 && compact.contains(&alias)) {
                MatchTier::Substring
            } else {
                continue;
            };
            let score = tier.base() + (200 - index.min(199) as u32);
            if best.is_none_or(|b| score > b.score) {
                best = Some(HeaderMatch { tier, score });
            }
        }
        best
    }

    /// The shape a column's values must have for the field to be plausible. A header alias
    /// can lie; values cannot, and they carry no language.
    pub(crate) fn value_shape(self) -> ValueShape {
        match self {
            ImportField::Date => ValueShape::Date,
            ImportField::Quantity
            | ImportField::Price
            | ImportField::Amount
            | ImportField::Fee
            | ImportField::Tax
            | ImportField::FxRate => ValueShape::Number,
            ImportField::Currency => ValueShape::CurrencyCode,
            ImportField::Isin => ValueShape::Isin,
            ImportField::LinkId => ValueShape::Link,
            _ => ValueShape::Free,
        }
    }
}

/// How many distinct import fields a row names: used to find the header line under a
/// broker's preamble, where the header is not the first row of the file.
pub(crate) fn header_row_score(cells: &[String]) -> usize {
    ImportField::ALL
        .iter()
        .filter(|field| cells.iter().any(|c| field.header_score(c).is_some()))
        .count()
}
