//! Value shapes, independent of language: a currency or ISIN column must look like one, a date need not.

use crate::import::parse::{parse_date_any, parse_decimal};

/// How literally a header names a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MatchTier {
    Substring,
    Word,
    Exact,
}

impl MatchTier {
    pub(super) fn base(self) -> u32 {
        match self {
            MatchTier::Substring => 1000,
            MatchTier::Word => 2000,
            MatchTier::Exact => 3000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaderMatch {
    pub tier: MatchTier,
    pub score: u32,
}

/// Shapes a column's values can be checked against without knowing any language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueShape {
    Date,
    Number,
    CurrencyCode,
    Isin,
    /// A pairing key: a value must occur twice, or it is a per-row id (see `paired_links`).
    Link,
    /// The opposite claim to `Link`: a broker's own row identifier is unique per row, so a
    /// column that repeats itself is a pairing key, not an id.
    Unique,
    Free,
}

impl ValueShape {
    /// Share of the values that fit the shape, `None` when there is nothing to judge.
    pub(crate) fn fit(self, values: &[&str]) -> Option<f32> {
        let values: Vec<&str> = values
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if values.is_empty() {
            return None;
        }
        if self == ValueShape::Free {
            return Some(1.0);
        }
        if self == ValueShape::Unique {
            let distinct: std::collections::BTreeSet<&str> = values.iter().copied().collect();
            return Some(if distinct.len() == values.len() { 1.0 } else { 0.0 });
        }
        // Repetition is the whole claim, so it is judged over the column rather than per value.
        if self == ValueShape::Link {
            let mut seen = std::collections::BTreeSet::new();
            return Some(if values.iter().any(|v| !seen.insert(*v)) {
                1.0
            } else {
                0.0
            });
        }
        // A code list repeats itself: a column of many distinct three-letter values is a
        // ticker column, not a currency column.
        if self == ValueShape::CurrencyCode {
            let distinct: std::collections::BTreeSet<String> =
                values.iter().map(|v| v.to_uppercase()).collect();
            if distinct.len() > (values.len() / 5).max(4) {
                return Some(0.0);
            }
        }
        let hits = values.iter().filter(|v| self.fits_value(v)).count();
        Some(hits as f32 / values.len() as f32)
    }

    fn fits_value(self, value: &str) -> bool {
        match self {
            // Judged over the whole column in `fit`; no single value is or is not a link.
            ValueShape::Free | ValueShape::Link | ValueShape::Unique => true,
            ValueShape::Date => parse_date_any(value).is_some(),
            // An ISIN parses as a number once the letters are dropped, so a number has to
            // start like one.
            ValueShape::Number => {
                let head = value.chars().next();
                head.is_some_and(|c| !c.is_alphabetic()) && parse_decimal(value, '.').is_some()
            }
            ValueShape::CurrencyCode => {
                value.chars().count() == 3 && value.chars().all(|c| c.is_alphabetic())
            }
            ValueShape::Isin => {
                let bytes = value.as_bytes();
                bytes.len() == 12
                    && bytes[..2].iter().all(|b| b.is_ascii_alphabetic())
                    && bytes[2..].iter().all(|b| b.is_ascii_alphanumeric())
                    && bytes[11].is_ascii_digit()
            }
        }
    }
}

/// Values veto a weak header match, never a literal one.
pub(super) fn shape_allows(shape: ValueShape, tier: MatchTier, values: &[&str]) -> bool {
    // A link overrules an exact header too, unlike every other shape: "Reference" names a
    // pairing key as literally as it names a row id, and only the values tell the two apart.
    if shape == ValueShape::Link || shape == ValueShape::Unique {
        return shape.fit(values).is_none_or(|fit| fit > 0.0);
    }
    if tier == MatchTier::Exact {
        return true;
    }
    match shape {
        ValueShape::Link | ValueShape::Unique => unreachable!("handled above, before the tier"),
        ValueShape::Free => {
            if tier == MatchTier::Word {
                return true;
            }
            [ValueShape::Date, ValueShape::Number, ValueShape::CurrencyCode]
                .iter()
                .all(|other| other.fit(values).is_none_or(|fit| fit < 0.8))
        }
        // Required even from a whole-word match; a date is exempt, its format may be unknown to us.
        ValueShape::CurrencyCode | ValueShape::Isin | ValueShape::Number => {
            shape.fit(values).is_none_or(|fit| fit >= 0.5)
        }
        ValueShape::Date => tier == MatchTier::Word || shape.fit(values).is_none_or(|fit| fit >= 0.5),
    }
}
