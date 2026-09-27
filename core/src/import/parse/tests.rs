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
    let csv = "date,type,amount\n2024-06-03,DEPOSIT,1000\n2024-06-04,DEPOSIT,20\nTransactions Total,,1020\n";
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
