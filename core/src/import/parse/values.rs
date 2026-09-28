//! Reading one cell as a date or a number, whatever shape the broker printed it in.

use chrono::NaiveDate;
use rust_decimal::Decimal;
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
