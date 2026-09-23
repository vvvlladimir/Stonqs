//! Instrument attributes as a CSV: an export of what is filled in, and an import that fills it.
//!
//! One row per instrument, one column per attribute. Instruments are joined the same way the
//! taxonomy import joins them — ISIN first, then the ticker — because an ISIN is the instrument
//! while a ticker is only one of its listings.

use super::parse::{ImportProblem, ParsedCsv, ProblemCode, Severity};
use super::taxonomy::{match_security, quote};
use crate::error::Result;
use crate::model::{AttributeKind, Security, SecurityAttributeDef};
use crate::storage::{AttributeValues, Store};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

/// Which columns name the instrument; everything else in the file is an attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeCsvConfig {
    pub symbol: Option<String>,
    pub isin: Option<String>,
    pub name: Option<String>,
    /// Attribute columns, by header, in the order the file prints them.
    pub attributes: Vec<String>,
}

/// One attribute column of the file, and what it would do to the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewAttribute {
    pub name: String,
    pub kind: AttributeKind,
    /// The attribute this column already is; `None` means the commit would create it.
    pub attribute_id: Option<String>,
    /// How many rows carry a value for it.
    pub values: usize,
}

/// One instrument's row: the values it would receive, and whether it was found at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeRow {
    pub row: usize,
    pub label: String,
    pub symbol: String,
    pub isin: String,
    pub security_id: Option<String>,
    /// `isin`, `symbol`, `symbol_base` or `name` — a code, never a label.
    pub matched_by: Option<String>,
    /// Attribute column name to the value it would be given. A blank cell is absent here: a
    /// file that names three attributes must not clear the other twelve.
    pub values: BTreeMap<String, String>,
}

/// The whole plan, written by nobody until [`commit_attributes`] is called.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributePreview {
    pub config: AttributeCsvConfig,
    pub attributes: Vec<PreviewAttribute>,
    pub rows: Vec<AttributeRow>,
    pub problems: Vec<ImportProblem>,
}

impl AttributePreview {
    pub fn matched(&self) -> usize {
        self.rows.iter().filter(|r| r.security_id.is_some()).count()
    }

    pub fn unmatched(&self) -> usize {
        self.rows.len() - self.matched()
    }

    /// Values that would be written, counting only rows that found their instrument.
    pub fn values(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.security_id.is_some())
            .map(|r| r.values.len())
            .sum()
    }

    pub fn new_attributes(&self) -> usize {
        self.attributes
            .iter()
            .filter(|a| a.attribute_id.is_none())
            .count()
    }
}

/// What the commit did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeImportResult {
    pub attributes_created: usize,
    pub instruments: usize,
    pub values: usize,
}

/// Reads the file's shape: the identifying columns by header meaning, the rest as attributes.
pub fn detect_attribute_config(parsed: &ParsedCsv) -> AttributeCsvConfig {
    let find = |keys: &[&str], deny: &[&str]| -> Option<String> {
        parsed
            .headers
            .iter()
            .find(|h| {
                let l = h.trim().to_lowercase();
                keys.iter().any(|k| l == *k || l.contains(k)) && !deny.iter().any(|d| l.contains(d))
            })
            .cloned()
    };

    let symbol = find(&["symbol", "ticker", "тикер", "символ"], &[]);
    let isin = find(&["isin"], &[]);
    let name = find(&["name", "наименование", "название"], &["file", "файл"]);

    let identifying = [&symbol, &isin, &name];
    let attributes = parsed
        .headers
        .iter()
        .filter(|h| !h.trim().is_empty())
        .filter(|h| !identifying.iter().any(|c| c.as_deref() == Some(h.as_str())))
        .cloned()
        .collect();

    AttributeCsvConfig {
        symbol,
        isin,
        name,
        attributes,
    }
}

/// Builds the plan from parsed rows, the instruments in the database and the attributes it
/// already defines. Pure: it reads no store, so the same file always previews the same way.
pub fn build_attribute_preview(
    parsed: &ParsedCsv,
    config: &AttributeCsvConfig,
    securities: &[Security],
    defs: &[SecurityAttributeDef],
) -> AttributePreview {
    let mut problems: Vec<ImportProblem> = parsed
        .problems
        .iter()
        .filter(|p| p.code != ProblemCode::BadDate)
        .cloned()
        .collect();

    let index_of = |column: &Option<String>| -> Option<usize> {
        column
            .as_ref()
            .and_then(|c| parsed.headers.iter().position(|h| h == c))
    };
    let symbol_idx = index_of(&config.symbol);
    let isin_idx = index_of(&config.isin);
    let name_idx = index_of(&config.name);

    if symbol_idx.is_none() && isin_idx.is_none() && name_idx.is_none() {
        problems.push(ImportProblem::file(
            ProblemCode::MissingColumn,
            "no column identifies the instrument (\"Symbol\", \"ISIN\", \"Name\")",
        ));
    }

    // A column already defined keeps its kind — an attribute's kind never changes (ADR-0031).
    let columns: Vec<(usize, PreviewAttribute)> = config
        .attributes
        .iter()
        .filter_map(|header| {
            let idx = parsed.headers.iter().position(|h| h == header)?;
            let existing = defs.iter().find(|d| d.name.eq_ignore_ascii_case(header));
            Some((
                idx,
                PreviewAttribute {
                    name: existing.map(|d| d.name.clone()).unwrap_or_else(|| header.clone()),
                    kind: existing
                        .map(|d| d.kind)
                        .unwrap_or_else(|| infer_kind(parsed, idx)),
                    attribute_id: existing.map(|d| d.id.clone()),
                    values: 0,
                },
            ))
        })
        .collect();
    let (indices, mut attributes): (Vec<usize>, Vec<PreviewAttribute>) = columns.into_iter().unzip();

    let mut rows = Vec::new();
    for (i, row) in parsed.rows.iter().enumerate() {
        let cell = |idx: Option<usize>| -> String {
            idx.and_then(|k| row.get(k))
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let symbol = cell(symbol_idx);
        let isin = cell(isin_idx);
        let label = cell(name_idx);
        if symbol.is_empty() && isin.is_empty() && label.is_empty() {
            continue;
        }

        let hit = match_security(&symbol, &isin, &label, securities);
        if hit.is_none() {
            problems.push(problem(
                i + 1,
                config.symbol.clone(),
                ProblemCode::UnknownSecurity,
                format!(
                    "\"{}\" is not in the database — this row is not written",
                    [label.as_str(), symbol.as_str(), isin.as_str()]
                        .iter()
                        .find(|v| !v.is_empty())
                        .copied()
                        .unwrap_or_default()
                ),
                Severity::Warning,
            ));
        }

        let mut values = BTreeMap::new();
        for (slot, idx) in indices.iter().enumerate() {
            let raw = row.get(*idx).map(|v| v.trim()).unwrap_or_default();
            if raw.is_empty() {
                continue;
            }
            let attribute = &attributes[slot];
            match attribute.kind.normalize(raw) {
                Ok(value) => {
                    values.insert(attribute.name.clone(), value);
                    attributes[slot].values += 1;
                }
                // The kind of an attribute that already exists is not up for negotiation, so a
                // cell that does not fit it is dropped with its reason, not silently stored.
                Err(e) => problems.push(problem(
                    i + 1,
                    Some(attribute.name.clone()),
                    ProblemCode::NotANumber,
                    format!("{e}; the cell is skipped"),
                    Severity::Error,
                )),
            }
        }

        rows.push(AttributeRow {
            row: i + 1,
            label,
            symbol,
            isin,
            security_id: hit.as_ref().map(|(s, _)| s.id.clone()),
            matched_by: hit.map(|(_, how)| how.to_string()),
            values,
        });
    }

    AttributePreview {
        config: config.clone(),
        attributes,
        rows,
        problems,
    }
}

/// Writes the plan: creates the attributes the file introduces, then fills the values.
///
/// A value already stored under an attribute the file does not name is left alone — the file is
/// a set of columns, not the whole state of the instrument.
pub fn commit_attributes(store: &Store, preview: &AttributePreview) -> Result<AttributeImportResult> {
    let mut result = AttributeImportResult::default();

    let mut defs = store.list_attribute_defs()?;
    let mut position = defs.iter().map(|d| d.position).max().unwrap_or(0);
    let mut id_of: BTreeMap<String, String> = BTreeMap::new();

    for attribute in &preview.attributes {
        if attribute.values == 0 {
            continue;
        }
        if let Some(id) = &attribute.attribute_id {
            id_of.insert(attribute.name.clone(), id.clone());
            continue;
        }
        // A rerun of the same file must not make a second attribute of the same name.
        if let Some(existing) = defs.iter().find(|d| d.name.eq_ignore_ascii_case(&attribute.name)) {
            id_of.insert(attribute.name.clone(), existing.id.clone());
            continue;
        }
        position += 1;
        let mut def = SecurityAttributeDef::new(attribute.name.clone(), attribute.kind);
        def.position = position;
        store.save_attribute_def(&def)?;
        id_of.insert(attribute.name.clone(), def.id.clone());
        defs.push(def);
        result.attributes_created += 1;
    }

    for row in &preview.rows {
        let Some(security_id) = row.security_id.as_deref() else {
            continue;
        };
        if row.values.is_empty() {
            continue;
        }
        let mut values: AttributeValues = store.attributes_for_security(security_id)?;
        let mut written = 0;
        for (name, value) in &row.values {
            let Some(id) = id_of.get(name) else { continue };
            values.insert(id.clone(), value.clone());
            written += 1;
        }
        if written == 0 {
            continue;
        }
        store.set_security_attributes(security_id, &values)?;
        result.instruments += 1;
        result.values += written;
    }

    Ok(result)
}

/// The instruments and their attributes as a CSV the same import reads back.
pub fn attributes_to_csv(
    securities: &[Security],
    defs: &[SecurityAttributeDef],
    values: &BTreeMap<String, AttributeValues>,
) -> String {
    let mut header: Vec<String> = vec!["Symbol".into(), "ISIN".into(), "Name".into()];
    header.extend(defs.iter().map(|d| quote(&d.name)));
    let mut out = header.join(",");
    out.push('\n');

    let mut rows: Vec<&Security> = securities.iter().collect();
    rows.sort_by_key(|a| a.symbol.to_lowercase());

    for security in rows {
        let mut line: Vec<String> = vec![
            quote(&security.symbol),
            quote(security.isin.as_deref().unwrap_or_default()),
            quote(&security.name),
        ];
        let stored = values.get(&security.id);
        for def in defs {
            let value = stored.and_then(|v| v.get(&def.id)).map(String::as_str);
            line.push(quote(value.unwrap_or_default()));
        }
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

/// A column nobody has defined yet is read from its own values: every cell a number makes a
/// number, every cell a date makes a date, and anything else is text.
fn infer_kind(parsed: &ParsedCsv, idx: usize) -> AttributeKind {
    let cells: Vec<&str> = parsed
        .rows
        .iter()
        .filter_map(|row| row.get(idx))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    if cells.is_empty() {
        return AttributeKind::Text;
    }
    if cells.iter().all(|v| Decimal::from_str(v).is_ok()) {
        return AttributeKind::Number;
    }
    if cells
        .iter()
        .all(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok())
    {
        return AttributeKind::Date;
    }
    AttributeKind::Text
}

fn problem(
    row: usize,
    column: Option<String>,
    code: ProblemCode,
    message: String,
    severity: Severity,
) -> ImportProblem {
    ImportProblem {
        row: Some(row),
        column,
        severity,
        code,
        message,
        params: Default::default(),
    }
}
