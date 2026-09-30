use super::*;
use crate::import::parse::{ParseConfig, parse_csv};
use crate::model::AssetKind;
use rust_decimal_macros::dec;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn assets() -> Vec<Asset> {
    let mut flat = Asset::new("Flat", AssetKind::Property, "EUR");
    flat.id = "house".into();
    let mut mortgage = Asset::new("Mortgage", AssetKind::Mortgage, "EUR");
    mortgage.id = "debt".into();
    vec![flat, mortgage]
}

fn preview(csv: &str, stored: &[AssetValue]) -> ValuesPreview {
    let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    build_values_preview(&parsed, &assets(), stored)
}

/// Columns are found by their headers, whatever order and spelling the file uses.
#[test]
fn the_three_columns_are_recognised_by_their_headers() {
    let plan = preview(
        "Valued on;Asset;Amount\n2026-01-15;Flat;400000\n2026-01-15;Mortgage;250000\n",
        &[],
    );

    assert_eq!(plan.config.date.as_deref(), Some("Valued on"));
    assert_eq!(plan.config.name.as_deref(), Some("Asset"));
    assert_eq!(plan.config.amount.as_deref(), Some("Amount"));
    assert_eq!(plan.writes(), 2);
    assert_eq!(plan.assets(), 2);
    assert!(plan.problems.is_empty(), "{:?}", plan.problems);
}

/// A name the portfolio does not hold is reported back, never created: a file cannot know what
/// kind of thing it is, nor which way its amount points.
#[test]
fn an_unknown_name_is_reported_and_nothing_is_invented() {
    let plan = preview(
        "asset,date,value\nDacha,2026-01-15,50000\nFlat,2026-01-15,400000\n",
        &[],
    );

    assert_eq!(plan.unmatched, vec!["Dacha".to_string()]);
    assert_eq!(plan.writes(), 1);
    assert_eq!(plan.rows.len(), 2);
}

/// The same file twice writes the same figures onto the same days, so the second run changes
/// nothing — and the preview says so before anything is written.
#[test]
fn a_row_for_a_day_already_answered_is_named_as_a_replacement() {
    let stored = vec![AssetValue::new("house", d(2026, 1, 15), dec!(390000))];
    let plan = preview(
        "asset,date,value\nFlat,2026-01-15,400000\nFlat,2026-06-30,410000\n",
        &stored,
    );

    assert_eq!(plan.replacements(), 1);
    assert!(plan.rows[0].replaces);
    assert!(!plan.rows[1].replaces);
}

/// Nothing in the model makes a name unique. Two things answering to one name means the file
/// cannot say which it means, so neither is written and the name is reported back.
#[test]
fn a_name_two_things_answer_to_is_refused_rather_than_guessed() {
    let mut first = Asset::new("Apartment", AssetKind::Property, "EUR");
    first.id = "flat-a".into();
    let mut second = Asset::new("apartment", AssetKind::Property, "EUR");
    second.id = "flat-b".into();
    let parsed = parse_csv(
        b"asset,date,value\nApartment,2026-01-15,400000\nApartment,2026-06-30,410000\n",
        &ParseConfig::default(),
    )
    .unwrap();

    let plan = build_values_preview(&parsed, &[first, second], &[]);

    assert_eq!(plan.ambiguous, vec!["Apartment".to_string()]);
    assert!(plan.unmatched.is_empty());
    assert_eq!(plan.writes(), 0);
    assert_eq!(plan.rows.len(), 2);
}

/// Two rows for one thing on one day: the commit writes the last over the first, so the preview
/// counts the second as a replacement rather than promising two figures for one day.
#[test]
fn a_day_answered_twice_inside_the_file_is_named_as_a_replacement() {
    let plan = preview(
        "asset,date,value\nFlat,2026-01-15,400000\nFlat,2026-01-15,410000\n",
        &[],
    );

    assert!(!plan.rows[0].replaces);
    assert!(plan.rows[1].replaces);
    assert_eq!(plan.replacements(), 1);
    // A warning, not an error: the row is still written, and the last figure is the one kept.
    let repeated: Vec<_> = plan
        .problems
        .iter()
        .filter(|p| p.code == ProblemCode::DuplicateInFile)
        .collect();
    assert_eq!(repeated.len(), 1, "{:?}", plan.problems);
    assert_eq!(repeated[0].row, Some(2));
    assert!(!repeated[0].is_error());
}

/// A debt written as a negative means the same thing by it; both sides are stored positive.
#[test]
fn a_negative_figure_is_read_as_what_is_owed() {
    let plan = preview("asset,date,value\nMortgage,2026-01-15,-250000\n", &[]);

    assert_eq!(plan.rows[0].amount, dec!(250000));
}

/// A cell that cannot be read stops its own row and no other, and says which cell it was.
#[test]
fn a_broken_cell_stops_one_row() {
    let plan = preview(
        "asset,date,value\nFlat,not-a-day,400000\nMortgage,2026-01-15,plenty\nFlat,2026-01-15,400000\n",
        &[],
    );

    assert_eq!(plan.writes(), 1);
    // The reader has its own opinions about a column of dates it could not parse, so the rows'
    // own problems are the ones asserted here.
    let mine: Vec<_> = plan.problems.iter().filter(|p| p.column.is_some()).collect();
    assert_eq!(mine.len(), 2, "{mine:?}");
    assert_eq!(mine[0].code, ProblemCode::BadDate);
    assert_eq!(mine[0].row, Some(1));
    assert_eq!(mine[1].code, ProblemCode::NotANumber);
    assert_eq!(mine[1].row, Some(2));
}

/// Without three columns there is nothing to read, and the file says that rather than half-doing it.
#[test]
fn a_file_missing_a_column_is_refused_whole() {
    let plan = preview("asset,value\nFlat,400000\n", &[]);

    assert!(plan.rows.is_empty());
    assert!(
        plan.problems.iter().any(|p| p.code == ProblemCode::MissingColumn),
        "{:?}",
        plan.problems
    );
}

/// Committing writes the matched rows and leaves the rest; a second commit is a no-op.
#[test]
fn committing_twice_writes_one_history() {
    let store = Store::open_in_memory().unwrap();
    let portfolio = crate::model::Portfolio::new("Main", "EUR");
    store.save_portfolio(&portfolio).unwrap();
    for asset in assets() {
        store.save_asset(&portfolio.id, &asset).unwrap();
    }

    let csv = "asset,date,value\nFlat,2026-01-15,400000\nDacha,2026-01-15,1\n";
    let plan = preview(csv, &[]);
    assert_eq!(commit_asset_values(&store, &plan).unwrap(), 1);
    assert_eq!(commit_asset_values(&store, &plan).unwrap(), 1);

    let stored = store.asset_values("house").unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].amount, dec!(400000));
}

/// The export is the import's own file: names with a comma in them survive the round trip.
#[test]
fn the_export_reads_back_as_the_same_figures() {
    let mut flat = Asset::new("Flat, upstairs", AssetKind::Property, "EUR");
    flat.id = "house".into();
    let values = vec![
        AssetValue::new("house", d(2026, 1, 15), dec!(400000)),
        AssetValue::new("house", d(2025, 1, 15), dec!(380000)),
    ];

    let csv = asset_values_to_csv(&[flat.clone()], &values);
    // Oldest first, and the name quoted because it holds a comma.
    assert!(csv.contains("\"Flat, upstairs\",2025-01-15,380000,EUR"), "{csv}");

    let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    let plan = build_values_preview(&parsed, &[flat], &[]);
    assert_eq!(plan.writes(), 2);
    assert_eq!(plan.rows[0].amount, dec!(380000));
}
