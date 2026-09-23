//! The CSV an export is written as. Semicolon-separated with a decimal comma, because that is
//! what a spreadsheet on a European locale opens without being asked twice.

use rust_decimal::Decimal;

/// A CSV cell. Semicolon-separated because that is what a Russian-locale Excel expects,
/// so a name carrying one — or a quote, or a newline — has to be quoted like any CSV field.
pub(crate) fn cell(value: &str) -> String {
    if value.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// One CSV row from cells that are already strings.
pub(crate) fn row<'a>(cells: impl IntoIterator<Item = &'a str>) -> String {
    let mut line = String::new();
    for (i, value) in cells.into_iter().enumerate() {
        if i > 0 {
            line.push(';');
        }
        line.push_str(&cell(value));
    }
    line.push('\n');
    line
}

/// A number for the same Excel the separator and the BOM are chosen for: a locale that
/// splits columns on `;` also reads `12.50` as text, so the decimal mark is a comma.
pub(crate) fn money(value: Decimal) -> String {
    value.to_string().replace('.', ",")
}

pub(crate) fn maybe(value: Option<Decimal>) -> String {
    value.map(money).unwrap_or_default()
}
