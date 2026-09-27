use super::mapping::{ImportField, header_row_score};
use crate::error::{Error, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

/// Date formats considered during automatic detection. Order decides ties, so the shape
/// seen more often in broker exports comes first (`%d/%m/%Y` before `%m/%d/%Y`).
pub const KNOWN_DATE_FORMATS: &[&str] = &[
    "%Y-%m-%d",
    "%d.%m.%Y",
    "%d/%m/%Y",
    "%m/%d/%Y",
    "%Y/%m/%d",
    "%d-%m-%Y",
    "%d/%m/%y",
    "%d.%m.%y",
    "%d-%b-%Y",
    "%d %b %Y",
    "%b %d, %Y",
    "%Y%m%d",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%d.%m.%Y %H:%M",
    "%d.%m.%Y %H:%M:%S",
    "%d/%m/%Y %H:%M:%S",
    "%m/%d/%Y %H:%M:%S",
    "%d-%m-%Y %H:%M:%S",
    "%d/%m/%y %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S%.fZ",
    "%Y-%m-%dT%H:%M:%S%.f%:z",
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%:z",
    "%b %d, %Y, %I:%M:%S %p",
];

/// Parsing options; `None` asks the parser to detect a value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseConfig {
    pub delimiter: Option<char>,

    pub skip_top_rows: usize,

    pub skip_bottom_rows: usize,

    pub has_header: Option<bool>,

    pub date_format: Option<String>,

    pub decimal_separator: Option<char>,
}

impl ParseConfig {
    pub fn has_header(&self) -> bool {
        self.has_header.unwrap_or(true)
    }
}

/// Whether a problem blocks importing a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Blocks the affected row or file from being imported.
    Error,

    /// Keeps the row importable while surfacing a plausibility concern.
    Warning,
}

/// Stable category for an import problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProblemCode {
    Encoding,

    MalformedRow,

    MissingColumn,

    NotANumber,

    BadDate,

    MissingValue,

    UnknownKind,

    UnknownAccount,

    WrongAccountKind,

    TransferWithSecurity,

    InvalidTransaction,

    DuplicateInStore,

    DuplicateInFile,

    RestatedInStore,

    SecurityWithoutSource,

    UnknownSecurity,

    DirectionFromSign,

    DirectionConflict,

    AmountSignAmbiguous,

    AmountBasisAmbiguous,

    AmountVsQuantityPrice,

    FeeExceedsAmount,

    FxRateOnBaseCurrency,

    SingleKindValue,

    FutureDate,

    ImplausibleDateSpan,

    ZeroAmount,

    SuspiciousCurrency,

    /// Shares arrived or left with no money named: the row moves a quantity at a price of zero,
    /// so the lot enters the portfolio with no cost basis and shows the whole holding as profit.
    DeliveryWithoutCost,

    /// The row is denominated in a currency the account it lands on does not keep.
    AccountCurrencyMismatch,

    /// The file's ticker names an instrument already stored under a different ISIN: one ticker,
    /// two instruments.
    TickerIsinConflict,

    /// A stored operation of the same day, account, instrument and quantity, differing only in
    /// what it is worth — what editing an imported row by hand leaves behind.
    SimilarInStore,

    /// One instrument's prices in the file step by a whole factor: a split the broker applied
    /// part-way through the statement.
    PossibleSplit,

    /// A sale or outgoing delivery takes more than the stored ledger and this file hold of the
    /// instrument at that date: the purchase or incoming transfer is missing.
    SaleExceedsHoldings,
}

/// File- or row-level diagnostic collected during parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProblem {
    pub row: Option<usize>,

    pub column: Option<String>,
    pub severity: Severity,
    pub code: ProblemCode,
    /// English wording, used as a fallback when the UI has no sentence for this code.
    pub message: String,
    /// Values the UI substitutes into its own wording, so the sentence can be translated.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

impl ImportProblem {
    pub fn file(code: ProblemCode, message: impl Into<String>) -> Self {
        ImportProblem {
            row: None,
            column: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn row(code: ProblemCode, row: usize, message: impl Into<String>) -> Self {
        ImportProblem {
            row: Some(row),
            column: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn cell(
        code: ProblemCode,
        row: usize,
        column: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ImportProblem {
            row: Some(row),
            column: Some(column.into()),
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    /// Adds one value the UI can put into its own sentence for this code.
    pub fn with(mut self, key: &str, value: impl ToString) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn warn(mut self) -> Self {
        self.severity = Severity::Warning;
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Parsed CSV values plus the detected configuration and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCsv {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,

    pub config: ParseConfig,
    pub problems: Vec<ImportProblem>,
}

impl ParsedCsv {
    pub fn value(&self, row: usize, column: &str) -> Option<&str> {
        let index = self.headers.iter().position(|h| h == column)?;
        self.rows.get(row)?.get(index).map(String::as_str)
    }

    pub fn column_values(&self, column: &str) -> Vec<&str> {
        let Some(index) = self.headers.iter().position(|h| h == column) else {
            return Vec::new();
        };
        self.rows
            .iter()
            .filter_map(|r| r.get(index))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect()
    }
}

/// Parses CSV bytes without converting values into model types.
pub fn parse_csv(content: &[u8], config: &ParseConfig) -> Result<ParsedCsv> {
    let mut problems = Vec::new();
    let text = decode(content, &mut problems);

    let delimiter = match config.delimiter {
        Some(d) => d,
        None => detect_delimiter(&text),
    };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter as u8)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());

    let mut records: Vec<Vec<String>> = Vec::new();
    for (i, record) in reader.records().enumerate() {
        match record {
            Ok(r) => records.push(r.iter().map(|s| s.trim().to_string()).collect()),
            Err(e) => problems.push(ImportProblem::row(
                ProblemCode::MalformedRow,
                i + 1,
                format!("the row could not be parsed: {e}"),
            )),
        }
    }

    records.retain(|r| !r.iter().all(|c| c.is_empty()));

    // Zero means "detect": a broker file often opens with a preamble and closes with a
    // total line, and neither is addressable before the rows are read.
    let mut config = config.clone();
    if config.has_header() && config.skip_top_rows == 0 {
        config.skip_top_rows = detect_header_row(&records);
        if config.skip_top_rows > 0 {
            problems.push(
                ImportProblem::file(
                    ProblemCode::MalformedRow,
                    format!(
                        "the header was found on row {}: the service rows above it are skipped",
                        config.skip_top_rows + 1
                    ),
                )
                .warn(),
            );
        }
    }
    if config.skip_bottom_rows == 0 {
        config.skip_bottom_rows = detect_footer_rows(&records);
        if config.skip_bottom_rows > 0 {
            problems.push(
                ImportProblem::file(
                    ProblemCode::MalformedRow,
                    format!(
                        "{} dateless rows were dropped from the bottom — they look like a report total",
                        config.skip_bottom_rows
                    ),
                )
                .warn(),
            );
        }
    }

    let start = config.skip_top_rows;
    let end = records.len().saturating_sub(config.skip_bottom_rows);
    if start >= end {
        return Err(Error::Invalid(format!(
            "after skipping {} rows at the top and {} at the bottom the file holds no data",
            config.skip_top_rows, config.skip_bottom_rows
        )));
    }
    let mut records: Vec<Vec<String>> = records[start..end].to_vec();

    let headers = if config.has_header() {
        let first = records.remove(0);

        first
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                if h.is_empty() {
                    format!("column {}", i + 1)
                } else {
                    h
                }
            })
            .collect::<Vec<String>>()
    } else {
        let width = records.iter().map(|r| r.len()).max().unwrap_or(0);
        (0..width).map(|i| format!("column {}", i + 1)).collect()
    };

    for (i, row) in records.iter_mut().enumerate() {
        if row.len() < headers.len() {
            row.resize(headers.len(), String::new());
        } else if row.len() > headers.len() {
            problems.push(
                ImportProblem::row(
                    ProblemCode::MalformedRow,
                    i + 1,
                    format!(
                        "the row holds {} columns against {} headers — the extra ones are dropped",
                        row.len(),
                        headers.len()
                    ),
                )
                .warn(),
            );
            row.truncate(headers.len());
        }
    }

    let mut resolved = config.clone();
    resolved.delimiter = Some(delimiter);
    resolved.has_header = Some(config.has_header());
    if resolved.decimal_separator.is_none() {
        resolved.decimal_separator = Some(detect_decimal_separator(&records));
    }
    if resolved.date_format.is_none() {
        match detect_date_format(&headers, &records) {
            Some((format, hits, total)) => {
                if hits < total {
                    problems.push(
                        ImportProblem::file(
                            ProblemCode::BadDate,
                            format!(
                                "the date format {format} fits {hits} of {total} rows; \
                                 the rest parse under other known formats"
                            ),
                        )
                        .warn(),
                    );
                }
                resolved.date_format = Some(format);
            }
            None => problems.push(ImportProblem::file(
                ProblemCode::BadDate,
                "the date format could not be detected — set it explicitly".to_string(),
            )),
        }
    }

    Ok(ParsedCsv {
        headers,
        rows: records,
        config: resolved,
        problems,
    })
}

/// BOM first, then valid UTF-8, then a guessed code page.
pub(super) fn decode(content: &[u8], problems: &mut Vec<ImportProblem>) -> String {
    let encoding = detect_encoding(content);
    let (text, _, had_errors) = encoding.decode(content);
    if had_errors {
        problems.push(
            ImportProblem::file(
                ProblemCode::Encoding,
                format!(
                    "some characters could not be decoded (encoding {}) and were replaced",
                    encoding.name()
                ),
            )
            .warn(),
        );
    }
    text.into_owned()
}

fn detect_encoding(content: &[u8]) -> &'static encoding_rs::Encoding {
    if let Some((encoding, _)) = encoding_rs::Encoding::for_bom(content) {
        return encoding;
    }
    if std::str::from_utf8(content).is_ok() {
        return encoding_rs::UTF_8;
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(content, true);
    detector.guess(None, true)
}

fn detect_delimiter(text: &str) -> char {
    let candidates = [',', ';', '\t'];
    let mut best = (',', 0usize);
    for delimiter in candidates {
        let counts: Vec<usize> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(20)
            .map(|l| l.matches(delimiter).count())
            .collect();
        let Some(&first) = counts.first() else { continue };
        if first == 0 {
            continue;
        }
        let consistent = counts.iter().filter(|c| **c == first).count();
        let score = first * consistent;
        if score > best.1 {
            best = (delimiter, score);
        }
    }
    best.0
}

fn detect_decimal_separator(rows: &[Vec<String>]) -> char {
    let (mut comma, mut dot) = (0usize, 0usize);
    for cell in rows.iter().flatten() {
        let cell = cell.trim();
        if cell.is_empty() || !cell.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }

        if KNOWN_DATE_FORMATS
            .iter()
            .any(|f| parse_date_with(cell, f).is_some())
        {
            continue;
        }
        let last_comma = cell.rfind(',');
        let last_dot = cell.rfind('.');
        match (last_comma, last_dot) {
            (Some(c), Some(d)) => {
                if c > d {
                    comma += 1
                } else {
                    dot += 1
                }
            }

            (Some(c), None) if cell.len() - c - 1 != 3 => comma += 1,
            (None, Some(d)) if cell.len() - d - 1 != 3 => dot += 1,
            _ => {}
        }
    }
    if comma > dot { ',' } else { '.' }
}

/// Detected format plus how many of the column's values it covers.
fn detect_date_format(headers: &[String], rows: &[Vec<String>]) -> Option<(String, usize, usize)> {
    for index in date_column_order(headers) {
        if let Some(found) = format_for_column(rows, index) {
            return Some(found);
        }
    }
    None
}

/// Finds the header line under a broker's preamble by counting how many import fields
/// each candidate row names. Returns how many rows to skip above it.
fn detect_header_row(records: &[Vec<String>]) -> usize {
    const WINDOW: usize = 10;
    let mut best = (records.first().map_or(0, |r| header_row_score(r)), 0usize);
    for (index, row) in records.iter().enumerate().take(WINDOW).skip(1) {
        let score = header_row_score(row);
        if score > best.0 {
            best = (score, index);
        }
    }
    // A header on the last line would leave no data at all — then it is not a header.
    if best.1 + 1 >= records.len() { 0 } else { best.1 }
}

/// A broker's closing "total" line carries no date; every real row does. Only trusted
/// when the row above it *is* dated, so an unrecognised date format never eats data.
fn detect_footer_rows(records: &[Vec<String>]) -> usize {
    const MAX: usize = 3;
    let dated = |row: &Vec<String>| row.iter().any(|c| parse_date_any(c).is_some());

    let mut count = 0;
    while count < MAX && count + 2 <= records.len() {
        if dated(&records[records.len() - 1 - count]) {
            break;
        }
        count += 1;
    }
    if count == 0 {
        return 0;
    }
    match records.get(records.len() - 1 - count) {
        Some(row) if dated(row) => count,
        _ => 0,
    }
}

fn date_column_order(headers: &[String]) -> Vec<usize> {
    let mut scored: Vec<(u32, usize)> = headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| ImportField::Date.header_score(h).map(|s| (s, i)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut order: Vec<usize> = scored.into_iter().map(|(_, i)| i).collect();
    let rest: Vec<usize> = (0..headers.len()).filter(|i| !order.contains(i)).collect();
    order.extend(rest);
    order
}

/// The format covering most values: one file can print two date shapes.
fn format_for_column(rows: &[Vec<String>], index: usize) -> Option<(String, usize, usize)> {
    let values: Vec<&str> = rows
        .iter()
        .filter_map(|r| r.get(index))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    if values.is_empty() {
        return None;
    }

    let mut best: Option<(&str, usize)> = None;
    for format in KNOWN_DATE_FORMATS {
        let hits = values
            .iter()
            .filter(|v| parse_date_with(v, format).is_some())
            .count();
        if hits > best.map_or(0, |(_, best_hits)| best_hits) {
            best = Some((format, hits));
        }
    }
    best.map(|(format, hits)| (format.to_string(), hits, values.len()))
}

pub fn parse_date_with(value: &str, format: &str) -> Option<NaiveDate> {
    let value = value.trim();
    if let Some(date) = parse_date_exact(value, format) {
        return Some(date);
    }
    // Schwab writes "10/14/2024 as of 10/10/2024": a date followed by a remark.
    let head = value.split_whitespace().next()?;
    if head == value {
        return None;
    }
    parse_date_exact(head, format)
}

/// Parses a date without knowing the file's format, returning the format that matched.
/// Used for the rows that disagree with the detected one.
pub fn parse_date_any(value: &str) -> Option<(NaiveDate, &'static str)> {
    KNOWN_DATE_FORMATS
        .iter()
        .find_map(|format| parse_date_with(value, format).map(|date| (date, *format)))
}

fn parse_date_exact(value: &str, format: &str) -> Option<NaiveDate> {
    if format.contains("%H") {
        chrono::NaiveDateTime::parse_from_str(value, format)
            .ok()
            .map(|dt| dt.date())
    } else {
        NaiveDate::parse_from_str(value, format).ok()
    }
}

/// A cell with no letter or digit ("-", "—") is an absent value, not a broken one.
pub fn is_placeholder(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.chars().any(char::is_alphanumeric)
}

pub fn parse_decimal(value: &str, decimal_separator: char) -> Option<Decimal> {
    // The file-level separator is only a tie-break: one export can mix "1.234,50" and
    // "1234.50", and trusting the file would turn -24.999996 into -24999996.
    let decimal_separator = cell_separator(value, decimal_separator);
    let mut cleaned = String::with_capacity(value.len());
    let mut negative = false;
    for c in value.trim().chars() {
        match c {
            '(' => negative = true,
            ')' | ' ' | '\u{00a0}' | '\'' => {}
            '-' => negative = true,
            '+' => {}
            c if c.is_ascii_digit() => cleaned.push(c),
            c if c == decimal_separator => cleaned.push('.'),

            ',' | '.' => {}

            _ => {}
        }
    }
    if cleaned.is_empty() || !cleaned.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let parsed = Decimal::from_str(&cleaned).ok()?;
    Some(if negative { -parsed } else { parsed })
}

/// Which of `,` / `.` separates the decimals of this very cell. Unambiguous on its own
/// whenever both appear, or the single one does not cut off a group of three digits.
fn cell_separator(value: &str, fallback: char) -> char {
    let commas = value.matches(',').count();
    let dots = value.matches('.').count();
    match (commas, dots) {
        (0, 0) => fallback,
        (_, 0) if commas > 1 => '.',
        (0, _) if dots > 1 => ',',
        (1, 0) => tail_separator(value, ',', fallback),
        (0, 1) => tail_separator(value, '.', fallback),
        _ => {
            if value.rfind(',') > value.rfind('.') {
                ','
            } else {
                '.'
            }
        }
    }
}

/// Exactly three digits behind the separator stays ambiguous ("1,234"): only there does
/// the file-level guess decide.
fn tail_separator(value: &str, candidate: char, fallback: char) -> char {
    let tail = value.rsplit(candidate).next().unwrap_or_default();
    let digits = tail.chars().filter(|c| c.is_ascii_digit()).count();
    if digits == 3 { fallback } else { candidate }
}

#[cfg(test)]
mod tests;
