use super::csv::{cell, maybe, money, row};
use rust_decimal_macros::dec;

/// A broker name carrying the separator must not become two columns.
#[test]
fn a_csv_cell_quotes_what_would_break_the_row() {
    assert_eq!(cell("Apple"), "Apple");
    assert_eq!(cell("Ford; Motor"), "\"Ford; Motor\"");
    assert_eq!(cell("ETF \"Core\""), "\"ETF \"\"Core\"\"\"");
    assert_eq!(cell("two\nlines"), "\"two\nlines\"");
    assert_eq!(
        row(["2024-01-01", "Ford; Motor", "12"]),
        "2024-01-01;\"Ford; Motor\";12\n"
    );
}

/// The decimal mark follows the separator: a comma never needs quoting in a `;` file.
#[test]
fn a_money_cell_carries_a_decimal_comma() {
    assert_eq!(money(dec!(1234.56)), "1234,56");
    assert_eq!(money(dec!(-7)), "-7");
    assert_eq!(cell(&money(dec!(1234.56))), "1234,56");
    assert_eq!(maybe(None), "");
}
