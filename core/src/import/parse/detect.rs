//! Guessing the settings a file does not state: encoding, delimiter, header and footer rows,
//! decimal separator and date format.

use super::values::{KNOWN_DATE_FORMATS, parse_date_any, parse_date_with};
use crate::import::mapping::{ImportField, header_row_score};

pub(super) fn detect_encoding(content: &[u8]) -> &'static encoding_rs::Encoding {
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

pub(super) fn detect_delimiter(text: &str) -> char {
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

pub(super) fn detect_decimal_separator(rows: &[Vec<String>]) -> char {
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
pub(super) fn detect_date_format(headers: &[String], rows: &[Vec<String>]) -> Option<(String, usize, usize)> {
    for index in date_column_order(headers) {
        if let Some(found) = format_for_column(rows, index) {
            return Some(found);
        }
    }
    None
}

/// Finds the header line under a broker's preamble by counting how many import fields
/// each candidate row names. Returns how many rows to skip above it.
pub(super) fn detect_header_row(records: &[Vec<String>]) -> usize {
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
pub(super) fn detect_footer_rows(records: &[Vec<String>]) -> usize {
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
