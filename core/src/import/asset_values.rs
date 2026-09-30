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
    /// Whether this row writes over a figure for the same thing on the same day — one already
    /// stored, or one an earlier row of this same file writes first.
    pub replaces: bool,
}

/// The plan, written by nobody until [`commit_asset_values`] is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValuesPreview {
    pub config: ValuesCsvConfig,
    pub rows: Vec<ValueRow>,
    /// Names the portfolio has no thing for, each once, in the order the file names them.
    pub unmatched: Vec<String>,
    /// Names the portfolio holds more than one thing under, each once. Nothing is written for
    /// them: two flats both called "Apartment" are told apart by nothing a name-only file
    /// carries, and picking either one writes the figure onto the wrong flat half the time.
    #[serde(default)]
    pub ambiguous: Vec<String>,
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
    let mut rows: Vec<ValueRow> = Vec::new();
    let mut unmatched: Vec<String> = Vec::new();
    let mut ambiguous: Vec<String> = Vec::new();

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
            ambiguous,
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

        let asset_id = resolve(&label, assets, &mut unmatched, &mut ambiguous);
        // A day is answered once: by a figure already stored, or by an earlier row of this same
        // file, which the commit then writes over. Counting only the stored ones would promise
        // two figures for a day that ends up holding one.
        let id = asset_id.as_deref();
        let in_store = id.is_some_and(|id| stored.iter().any(|v| v.asset_id == id && v.date == date));
        let in_file = id.is_some_and(|id| {
            rows.iter()
                .any(|r| r.asset_id.as_deref() == Some(id) && r.date == date)
        });
        if in_file {
            problems.push(
                ImportProblem::cell(
                    ProblemCode::DuplicateInFile,
                    number,
                    date_column.clone(),
                    "the file already answers this day for this thing; the last figure wins",
                )
                .warn(),
            );
        }
        let replaces = in_store || in_file;

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
        ambiguous,
        problems,
    }
}

/// The thing a row names, if the portfolio holds exactly one under that name. Names that found
/// nothing and names that found several are kept apart: they ask the owner for different things.
fn resolve(
    label: &str,
    assets: &[Asset],
    unmatched: &mut Vec<String>,
    ambiguous: &mut Vec<String>,
) -> Option<String> {
    match match_asset(assets, label) {
        NameMatch::One(asset) => Some(asset.id.clone()),
        NameMatch::Many => {
            remember(ambiguous, label);
            None
        }
        NameMatch::None => {
            remember(unmatched, label);
            None
        }
    }
}

/// Adds a name to a list it is not on yet, spelling and case as the file wrote it the first time.
fn remember(names: &mut Vec<String>, label: &str) {
    if !names.iter().any(|seen| seen.eq_ignore_ascii_case(label)) {
        names.push(label.to_string());
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

/// What a name in the file found in the portfolio.
enum NameMatch<'a> {
    One(&'a Asset),
    /// Two things or more answer to it. Nothing in the model makes a name unique, so this is a
    /// state the portfolio can really be in.
    Many,
    None,
}

/// Exactly the name, ignoring case and surrounding space. No fuzzy match, and no first-one-wins:
/// writing a figure onto the wrong flat is worse than reporting a name back.
fn match_asset<'a>(assets: &'a [Asset], label: &str) -> NameMatch<'a> {
    let mut found = assets
        .iter()
        .filter(|asset| asset.name.trim().eq_ignore_ascii_case(label.trim()));
    match (found.next(), found.next()) {
        (Some(asset), None) => NameMatch::One(asset),
        (Some(_), Some(_)) => NameMatch::Many,
        _ => NameMatch::None,
    }
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
