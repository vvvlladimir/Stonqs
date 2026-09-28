use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

mod detect;
mod problem;
mod values;

use detect::{
    detect_date_format, detect_decimal_separator, detect_delimiter, detect_encoding, detect_footer_rows,
    detect_header_row,
};
pub use problem::{ImportProblem, ProblemCode, Severity};
pub use values::{KNOWN_DATE_FORMATS, is_placeholder, parse_date_any, parse_date_with, parse_decimal};

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
    let delimiter = config.delimiter.unwrap_or_else(|| detect_delimiter(&text));
    let records = read_records(&text, delimiter, &mut problems);

    let mut config = config.clone();
    let mut records = frame(&mut config, records, &mut problems)?;
    let headers = if config.has_header() {
        name_headers(records.remove(0))
    } else {
        let width = records.iter().map(|r| r.len()).max().unwrap_or(0);
        (0..width).map(|i| format!("column {}", i + 1)).collect()
    };
    fit_to_headers(&mut records, headers.len(), &mut problems);

    config.delimiter = Some(delimiter);
    config.has_header = Some(config.has_header());
    resolve_values(&mut config, &headers, &records, &mut problems);

    Ok(ParsedCsv {
        headers,
        rows: records,
        config,
        problems,
    })
}

/// Every non-empty record, cells trimmed; a record the reader cannot parse is a problem, not an end.
fn read_records(text: &str, delimiter: char, problems: &mut Vec<ImportProblem>) -> Vec<Vec<String>> {
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
    records
}

/// Cuts the preamble above the header and the total line below the data. Zero means "detect":
/// neither is addressable before the rows are read.
fn frame(
    config: &mut ParseConfig,
    records: Vec<Vec<String>>,
    problems: &mut Vec<ImportProblem>,
) -> Result<Vec<Vec<String>>> {
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
    Ok(records[start..end].to_vec())
}

fn name_headers(first: Vec<String>) -> Vec<String> {
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
        .collect()
}

/// Pads a short row with empty cells and drops the extra cells of a long one.
fn fit_to_headers(records: &mut [Vec<String>], width: usize, problems: &mut Vec<ImportProblem>) {
    for (i, row) in records.iter_mut().enumerate() {
        if row.len() < width {
            row.resize(width, String::new());
        } else if row.len() > width {
            problems.push(
                ImportProblem::row(
                    ProblemCode::MalformedRow,
                    i + 1,
                    format!(
                        "the row holds {} columns against {width} headers — the extra ones are dropped",
                        row.len(),
                    ),
                )
                .warn(),
            );
            row.truncate(width);
        }
    }
}

/// Detects the decimal separator and date format the caller left open.
fn resolve_values(
    config: &mut ParseConfig,
    headers: &[String],
    records: &[Vec<String>],
    problems: &mut Vec<ImportProblem>,
) {
    if config.decimal_separator.is_none() {
        config.decimal_separator = Some(detect_decimal_separator(records));
    }
    if config.date_format.is_some() {
        return;
    }
    match detect_date_format(headers, records) {
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
            config.date_format = Some(format);
        }
        None => problems.push(ImportProblem::file(
            ProblemCode::BadDate,
            "the date format could not be detected — set it explicitly".to_string(),
        )),
    }
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

#[cfg(test)]
mod tests;
