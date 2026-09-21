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

    SecurityWithoutSource,

    UnknownSecurity,

    DirectionFromSign,

    DirectionConflict,

    AmountSignAmbiguous,

    AmountVsQuantityPrice,

    FeeExceedsAmount,

    FxRateOnBaseCurrency,

    SingleKindValue,

    FutureDate,

    ImplausibleDateSpan,

    ZeroAmount,

    SuspiciousCurrency,
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

/// A broker export is not necessarily UTF-8 — German exports are routinely CP1252 or
/// windows-1250 — and the user cannot be asked to re-save the file. BOM decides when
/// present, otherwise valid UTF-8 wins and only then is the code page guessed.
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

/// The format covering the most values of the column, with that count and the column's
/// total. A broker changes its export shape mid-history — Trade Republic prints both
/// `2024-11-30` and `2025-01-16T16:13:36` in one file — so demanding one format for the
/// whole column detects nothing at all.
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

/// A cell that stands for "nothing here": brokers print "-", "—" or "--" where a number is
/// not applicable. Any real number in any locale carries a digit, so a cell without a single
/// letter or digit is an absent value rather than a broken one.
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
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn detects_semicolon_and_german_decimals() {
        let csv = "Datum;Typ;Stück;Kurs\n03.06.2024;Kauf;10;1.234,50\n04.06.2024;Kauf;5;99,90\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.config.delimiter, Some(';'));
        assert_eq!(parsed.config.decimal_separator, Some(','));
        assert_eq!(parsed.config.date_format.as_deref(), Some("%d.%m.%Y"));
        assert_eq!(parsed.headers.len(), 4);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.value(0, "Kurs"), Some("1.234,50"));
    }

    #[test]
    fn quoted_commas_do_not_split_columns() {
        let csv = "date,type,note\n2024-06-03,BUY,\"bought 10, sold 0\"\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.headers.len(), 3);
        assert_eq!(parsed.value(0, "note"), Some("bought 10, sold 0"));
    }

    #[test]
    fn skips_broker_preamble_and_footer() {
        let csv = "Broker statement\nAccount 12345\ndate,type,amount\n2024-06-03,DEPOSIT,1000\nTotal,,1000\n";
        let config = ParseConfig {
            skip_top_rows: 2,
            skip_bottom_rows: 1,
            ..ParseConfig::default()
        };
        let parsed = parse_csv(csv.as_bytes(), &config).unwrap();
        assert_eq!(parsed.headers, vec!["date", "type", "amount"]);
        assert_eq!(parsed.rows.len(), 1);
    }

    #[test]
    fn date_format_is_checked_against_the_whole_column() {
        let csv = "date,amount\n03/04/2024,10\n25/12/2024,20\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();

        assert_eq!(parsed.config.date_format.as_deref(), Some("%d/%m/%Y"));
    }

    #[test]
    fn one_column_may_carry_two_date_formats() {
        // Trade Republic switched its export shape mid-history: the same column holds
        // "2024-11-30" and "2025-01-16T16:13:36". The majority wins, the rest fall back.
        let csv = "date,amount\n2024-11-30,1\n2024-11-26,2\n2024-07-23,3\n2025-01-16T16:13:36,4\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.config.date_format.as_deref(), Some("%Y-%m-%d"));
        assert!(
            parsed
                .problems
                .iter()
                .any(|p| p.code == ProblemCode::BadDate && p.severity == Severity::Warning)
        );
        assert_eq!(
            parse_date_any("2025-01-16T16:13:36").map(|(d, _)| d),
            Some(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap())
        );
    }

    #[test]
    fn the_files_decimal_separator_does_not_corrupt_a_row_written_the_other_way() {
        // Same file, both shapes: trusting the file-level guess would read -24999996.
        assert_eq!(parse_decimal("-10,84", ','), Some(dec!(-10.84)));
        assert_eq!(parse_decimal("-24.999996", ','), Some(dec!(-24.999996)));
        assert_eq!(parse_decimal("0,052361", ','), Some(dec!(0.052361)));
        assert_eq!(parse_decimal("86.714375", ','), Some(dec!(86.714375)));
        // Exactly three digits behind the separator stays ambiguous — the file decides.
        assert_eq!(parse_decimal("1,234", ','), Some(dec!(1.234)));
        assert_eq!(parse_decimal("1,234", '.'), Some(dec!(1234)));
    }

    #[test]
    fn a_total_line_at_the_bottom_needs_no_setting() {
        let csv =
            "date,type,amount\n2024-06-03,DEPOSIT,1000\n2024-06-04,DEPOSIT,20\nTransactions Total,,1020\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.config.skip_bottom_rows, 1);
        assert_eq!(parsed.rows.len(), 2);
    }

    #[test]
    fn the_last_row_survives_when_the_date_format_is_unknown_to_us() {
        // Every row undated: that is an unrecognised format, not a footer — eating the
        // last row here would silently drop data.
        let csv = "date,type,amount\nyesterday,DEPOSIT,1000\ntoday,DEPOSIT,20\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.config.skip_bottom_rows, 0);
        assert_eq!(parsed.rows.len(), 2);
    }

    #[test]
    fn the_header_is_found_under_a_brokers_preamble() {
        // Directa opens with six lines about the account before the header.
        let csv = "Conto : CONTO COGNOME NOME,,\nData estrazione : 2-1-2025 11:58:30,,\n\
                   Ticker,Isin,Importo euro\nIEMB,IE00B2NPKV68,20.95\n";
        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        assert_eq!(parsed.config.skip_top_rows, 2);
        assert_eq!(parsed.headers, vec!["Ticker", "Isin", "Importo euro"]);
        assert_eq!(parsed.rows.len(), 1);
    }

    #[test]
    fn broker_date_shapes_beyond_the_obvious_ones() {
        let day = |y, m, d| Some(NaiveDate::from_ymd_opt(y, m, d).unwrap());
        // IBKR writes dates without separators, Saxo spells the month.
        assert_eq!(parse_date_any("20230623").map(|(d, _)| d), day(2023, 6, 23));
        assert_eq!(parse_date_any("02-Apr-2025").map(|(d, _)| d), day(2025, 4, 2));
        assert_eq!(
            parse_date_any("May 5, 2020, 10:10:57 PM").map(|(d, _)| d),
            day(2020, 5, 5)
        );
        // Schwab appends a remark to the date.
        assert_eq!(
            parse_date_with("10/14/2024 as of 10/10/2024", "%m/%d/%Y"),
            day(2024, 10, 14)
        );
    }

    #[test]
    fn a_file_in_a_legacy_code_page_is_read_without_asking_the_user() {
        // A German export saved as windows-1252: "Gebühr" carries 0xFC where UTF-8 needs two
        // bytes. Telling the user to re-save the file is not an answer.
        let mut bytes: Vec<u8> = b"Datum;Typ;Geb".to_vec();
        bytes.push(0xFC);
        bytes.extend_from_slice(b"hr\n03.06.2024;Kauf;1,50\n04.06.2024;Verkauf;2,50\n");
        let parsed = parse_csv(&bytes, &ParseConfig::default()).unwrap();
        assert_eq!(parsed.headers, vec!["Datum", "Typ", "Gebühr"]);
        assert!(parsed.problems.iter().all(|p| p.code != ProblemCode::Encoding));
    }

    #[test]
    fn a_dash_is_an_absent_number_not_a_broken_one() {
        // eToro prints "-" where a field does not apply.
        assert!(is_placeholder("-"));
        assert!(is_placeholder("—"));
        assert!(is_placeholder("--"));
        assert!(!is_placeholder(""));
        assert!(!is_placeholder("n/a"), "letters carry meaning and stay an error");
        assert!(!is_placeholder("0"));
    }

    #[test]
    fn parses_money_written_the_way_brokers_write_it() {
        assert_eq!(parse_decimal("1.234,50", ','), Some(dec!(1234.50)));
        assert_eq!(parse_decimal("1,234.50", '.'), Some(dec!(1234.50)));
        assert_eq!(parse_decimal("12.50 USD", '.'), Some(dec!(12.50)));
        assert_eq!(parse_decimal("(1 234,56)", ','), Some(dec!(-1234.56)));
        assert_eq!(parse_decimal("-99", '.'), Some(dec!(-99)));
        assert_eq!(parse_decimal("", '.'), None);
        assert_eq!(parse_decimal("n/a", '.'), None);
    }
}
