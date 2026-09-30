//! Valuations of things owned and owed as CSV: one row per thing per day.
//!
//! Joined by name, because these things have no ISIN, no ticker and no account number — a name is
//! all there ever is. Re-running the same file writes the same figures over the same days, so it
//! is a no-op rather than a second history (ADR-0092). Nothing is created: a name the portfolio
//! does not hold is reported, never invented, because guessing what kind of thing it is — and
//! which way its amount points — is not something a file can settle.

use super::parse::{ImportProblem, ParsedCsv, ProblemCode, parse_date_any, parse_decimal};
use crate::error::Result;
use crate::model::{Asset, AssetValue};
use crate::storage::Store;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Header words that name each column, lower-cased. First match in the file's own order wins.
const NAME_WORDS: &[&str] = &["asset", "thing", "name", "item", "liability", "debt"];
const DATE_WORDS: &[&str] = &["date", "day", "as of", "valued"];
const AMOUNT_WORDS: &[&str] = &["value", "amount", "balance", "worth", "owed"];

/// Which columns the file was read by; carried so the answer can say what it used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValuesCsvConfig {
    pub name: Option<String>,
    pub date: Option<String>,
    pub amount: Option<String>,
}

/// One row of the file, matched or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueRow {
    pub row: usize,
    /// The name as the file spells it.
    pub label: String,
    pub asset_id: Option<String>,
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    /// Whether a figure for this thing on this day is already stored — the row replaces it.
    pub replaces: bool,
}

/// The plan, written by nobody until [`commit_asset_values`] is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValuesPreview {
    pub config: ValuesCsvConfig,
    pub rows: Vec<ValueRow>,
    /// Names the portfolio has no thing for, each once, in the order the file names them.
    pub unmatched: Vec<String>,
    pub problems: Vec<ImportProblem>,
}

impl ValuesPreview {
    /// Rows that would be written: the matched ones.
    pub fn writes(&self) -> usize {
        self.rows.iter().filter(|row| row.asset_id.is_some()).count()
    }

    /// How many of those replace a figure already stored for that thing on that day.
    pub fn replacements(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.asset_id.is_some() && row.replaces)
            .count()
    }

    /// How many distinct things the file speaks about and the portfolio knows.
    pub fn assets(&self) -> usize {
        self.rows
            .iter()
            .filter_map(|row| row.asset_id.as_deref())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// Reads the file against the portfolio's things. `stored` is every valuation already written, so
/// a row that replaces one can say so before anything is committed.
pub fn build_values_preview(parsed: &ParsedCsv, assets: &[Asset], stored: &[AssetValue]) -> ValuesPreview {
    let config = detect_values_config(parsed);
    let mut problems = parsed.problems.clone();
    let mut rows = Vec::new();
    let mut unmatched: Vec<String> = Vec::new();

    let (Some(name_column), Some(date_column), Some(amount_column)) =
        (&config.name, &config.date, &config.amount)
    else {
        problems.push(ImportProblem::file(
            ProblemCode::MissingColumn,
            "the file needs a name, a date and a value column",
        ));
        return ValuesPreview {
            config,
            rows,
            unmatched,
            problems,
        };
    };

    for index in 0..parsed.rows.len() {
        let number = index + 1;
        let label = parsed
            .value(index, name_column)
            .unwrap_or_default()
            .trim()
            .to_string();
        if label.is_empty() {
            problems.push(ImportProblem::cell(
                ProblemCode::MissingValue,
                number,
                name_column.clone(),
                "the row names nothing",
            ));
            continue;
        }
        let Some((date, _)) = parsed
            .value(index, date_column)
            .and_then(|value| parse_date_any(value.trim()))
        else {
            problems.push(ImportProblem::cell(
                ProblemCode::BadDate,
                number,
                date_column.clone(),
                "the day could not be read",
            ));
            continue;
        };
        let Some(amount) = parsed
            .value(index, amount_column)
            .and_then(|value| parse_decimal(value.trim(), '.'))
        else {
            problems.push(ImportProblem::cell(
                ProblemCode::NotANumber,
                number,
                amount_column.clone(),
                "the figure could not be read",
            ));
            continue;
        };
        // Both sides are stored positive; a file that writes a debt as a minus means the same
        // thing by it, and refusing the row over a sign would help nobody.
        let amount = amount.abs();

        let asset_id = match_asset(assets, &label).map(|asset| asset.id.clone());
        if asset_id.is_none() && !unmatched.iter().any(|seen| seen.eq_ignore_ascii_case(&label)) {
            unmatched.push(label.clone());
        }
        let replaces = asset_id.as_deref().is_some_and(|id| {
            stored
                .iter()
                .any(|value| value.asset_id == id && value.date == date)
        });

        rows.push(ValueRow {
            row: number,
            label,
            asset_id,
            date,
            amount,
            replaces,
        });
    }

    ValuesPreview {
        config,
        rows,
        unmatched,
        problems,
    }
}

/// Writes every matched row. One figure per thing per day, so a repeated run changes nothing.
pub fn commit_asset_values(store: &Store, preview: &ValuesPreview) -> Result<usize> {
    let mut written = 0;
    for row in &preview.rows {
        let Some(asset_id) = row.asset_id.as_deref() else {
            continue;
        };
        store.save_asset_value(&AssetValue::new(asset_id, row.date, row.amount))?;
        written += 1;
    }
    Ok(written)
}

/// The columns, by the first header that reads like each one.
pub fn detect_values_config(parsed: &ParsedCsv) -> ValuesCsvConfig {
    let find = |words: &[&str]| -> Option<String> {
        parsed
            .headers
            .iter()
            .find(|header| {
                let lower = header.to_lowercase();
                words.iter().any(|word| lower.contains(word))
            })
            .cloned()
    };
    // The date column is looked for before the name one: "valued on" reads as both, and a file
    // with a single such column is talking about a day.
    let date = find(DATE_WORDS);
    let name = parsed
        .headers
        .iter()
        .find(|header| {
            let lower = header.to_lowercase();
            Some(*header) != date.as_ref() && NAME_WORDS.iter().any(|word| lower.contains(word))
        })
        .cloned();
    let amount = parsed
        .headers
        .iter()
        .find(|header| {
            let lower = header.to_lowercase();
            Some(*header) != date.as_ref()
                && Some(*header) != name.as_ref()
                && AMOUNT_WORDS.iter().any(|word| lower.contains(word))
        })
        .cloned();
    ValuesCsvConfig { name, date, amount }
}

/// Exactly the name, ignoring case and surrounding space. No fuzzy match: writing a figure onto
/// the wrong flat is worse than reporting a name back.
fn match_asset<'a>(assets: &'a [Asset], label: &str) -> Option<&'a Asset> {
    assets
        .iter()
        .find(|asset| asset.name.trim().eq_ignore_ascii_case(label.trim()))
}

/// Every valuation of every thing as one CSV — the file this import reads back.
pub fn asset_values_to_csv(assets: &[Asset], values: &[AssetValue]) -> String {
    let mut out = String::from("asset,date,value,currency\n");
    let mut rows: Vec<(&Asset, &AssetValue)> = values
        .iter()
        .filter_map(|value| {
            assets
                .iter()
                .find(|asset| asset.id == value.asset_id)
                .map(|asset| (asset, value))
        })
        .collect();
    rows.sort_by(|(a, left), (b, right)| a.name.cmp(&b.name).then(left.date.cmp(&right.date)));

    for (asset, value) in rows {
        out.push_str(&format!(
            "{},{},{},{}\n",
            quote(&asset.name),
            value.date,
            value.amount,
            asset.currency
        ));
    }
    out
}

/// A name with a comma or a quote in it has to survive the round trip.
fn quote(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests;
