//! Instrument attributes as a CSV: what a file fills in, what it refuses, and what it leaves alone.

use sq_core::import::{
    ParseConfig, attributes_to_csv, build_attribute_preview, commit_attributes, detect_attribute_config,
    parse_csv,
};
use sq_core::model::{AttributeKind, Security, SecurityAttributeDef, SecurityKind};
use sq_core::storage::Store;

const FILE: &str = "\
Symbol,ISIN,Name,TER,Domicile,Inception
VWCE,IE00BK5BQT80,FTSE All-World,0.22,Ireland,2019-07-23
IWDA,IE00B4L5Y983,MSCI World,0.20,Ireland,2009-09-25
NOPE,XX0000000000,Nothing we hold,0.10,Nowhere,2020-01-01
";

fn seeded() -> (Store, Vec<Security>) {
    let store = Store::open_in_memory().unwrap();
    let mut securities = vec![
        Security::new("VWCE", "FTSE All-World", "EUR", SecurityKind::Etf),
        Security::new("IWDA", "MSCI World", "USD", SecurityKind::Etf),
    ];
    securities[0].isin = Some("IE00BK5BQT80".into());
    securities[1].isin = Some("IE00B4L5Y983".into());
    for security in &securities {
        store.save_security(security).unwrap();
    }
    (store, securities)
}

/// A file naming three columns nobody defined yet creates three attributes, each read from its
/// own values: 0.22 is a number, 2019-07-23 a date, Ireland text.
#[test]
fn a_new_column_becomes_an_attribute_of_the_kind_its_values_are() {
    let (store, securities) = seeded();
    let parsed = parse_csv(FILE.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    assert_eq!(config.attributes, ["TER", "Domicile", "Inception"]);

    let preview = build_attribute_preview(&parsed, &config, &securities, &[]);
    let kind = |name: &str| {
        preview
            .attributes
            .iter()
            .find(|a| a.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the preview"))
            .kind
    };
    assert_eq!(kind("TER"), AttributeKind::Number);
    assert_eq!(kind("Inception"), AttributeKind::Date);
    assert_eq!(kind("Domicile"), AttributeKind::Text);

    // Two of the three rows are instruments we hold; the third is reported, not written.
    assert_eq!(preview.matched(), 2);
    assert_eq!(preview.unmatched(), 1);
    assert_eq!(preview.values(), 6);
    assert_eq!(preview.new_attributes(), 3);

    let result = commit_attributes(&store, &preview).unwrap();
    assert_eq!(result.attributes_created, 3);
    assert_eq!(result.instruments, 2);
    assert_eq!(result.values, 6);

    let defs = store.list_attribute_defs().unwrap();
    let ter = defs.iter().find(|d| d.name == "TER").unwrap();
    let values = store.attributes_for_security(&securities[0].id).unwrap();
    // 0.22 stored as the number it is, not as the text "0.22 ".
    assert_eq!(values.get(&ter.id).map(String::as_str), Some("0.22"));
}

/// Re-importing the same file changes nothing: no second attribute of the same name, no new rows.
#[test]
fn importing_the_same_file_twice_is_a_no_op() {
    let (store, securities) = seeded();
    let parsed = parse_csv(FILE.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);

    let first = build_attribute_preview(&parsed, &config, &securities, &[]);
    commit_attributes(&store, &first).unwrap();
    let after_first = store.list_attribute_defs().unwrap();

    let defs = store.list_attribute_defs().unwrap();
    let second = build_attribute_preview(&parsed, &config, &securities, &defs);
    assert_eq!(second.new_attributes(), 0);
    let result = commit_attributes(&store, &second).unwrap();
    assert_eq!(result.attributes_created, 0);

    assert_eq!(store.list_attribute_defs().unwrap().len(), after_first.len());
    let values = store.attributes_for_security(&securities[0].id).unwrap();
    assert_eq!(values.len(), 3);
}

/// An attribute's kind never changes, so a cell that does not fit it is refused — the cell, not
/// the row: the instrument's other columns are still written.
#[test]
fn a_value_that_does_not_fit_an_existing_kind_is_refused_alone() {
    let (store, securities) = seeded();
    let ter = SecurityAttributeDef::new("TER", AttributeKind::Number);
    store.save_attribute_def(&ter).unwrap();

    let file = "\
Symbol,TER,Domicile
VWCE,cheap,Ireland
IWDA,0.20,Ireland
";
    let parsed = parse_csv(file.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    let defs = store.list_attribute_defs().unwrap();
    let preview = build_attribute_preview(&parsed, &config, &securities, &defs);

    // The kind is the stored one, not inferred from a file that disagrees with it.
    let column = preview.attributes.iter().find(|a| a.name == "TER").unwrap();
    assert_eq!(column.kind, AttributeKind::Number);
    assert_eq!(column.attribute_id.as_deref(), Some(ter.id.as_str()));

    assert_eq!(preview.problems.len(), 1);
    assert_eq!(preview.problems[0].row, Some(1));
    // The row keeps its other value; only the cell is dropped.
    assert_eq!(preview.rows[0].values.len(), 1);
    assert!(preview.rows[0].values.contains_key("Domicile"));

    commit_attributes(&store, &preview).unwrap();
    let defs = store.list_attribute_defs().unwrap();
    let domicile = defs.iter().find(|d| d.name == "Domicile").unwrap();
    let values = store.attributes_for_security(&securities[0].id).unwrap();
    assert_eq!(values.get(&domicile.id).map(String::as_str), Some("Ireland"));
    assert_eq!(values.get(&ter.id), None);
}

/// A file naming one column does not clear the attributes it says nothing about.
#[test]
fn a_column_the_file_does_not_name_is_left_alone() {
    let (store, securities) = seeded();
    let domicile = SecurityAttributeDef::new("Domicile", AttributeKind::Text);
    store.save_attribute_def(&domicile).unwrap();
    store
        .set_security_attributes(
            &securities[0].id,
            &[(domicile.id.clone(), "Ireland".to_string())]
                .into_iter()
                .collect(),
        )
        .unwrap();

    let file = "Symbol,TER\nVWCE,0.22\n";
    let parsed = parse_csv(file.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    let defs = store.list_attribute_defs().unwrap();
    let preview = build_attribute_preview(&parsed, &config, &securities, &defs);
    commit_attributes(&store, &preview).unwrap();

    let values = store.attributes_for_security(&securities[0].id).unwrap();
    assert_eq!(values.get(&domicile.id).map(String::as_str), Some("Ireland"));
    assert_eq!(values.len(), 2);
}

/// ISIN identifies the instrument, so a file printing another venue's ticker still lands right.
#[test]
fn an_isin_outranks_a_foreign_ticker() {
    let (_store, securities) = seeded();
    let file = "Symbol,ISIN,Domicile\nIWDA.L,IE00B4L5Y983,Ireland\n";
    let parsed = parse_csv(file.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    let preview = build_attribute_preview(&parsed, &config, &securities, &[]);

    assert_eq!(
        preview.rows[0].security_id.as_deref(),
        Some(securities[1].id.as_str())
    );
    assert_eq!(preview.rows[0].matched_by.as_deref(), Some("isin"));
}

/// The export writes what the import reads: a round trip leaves the same values in place.
#[test]
fn the_export_is_read_back_by_the_import() {
    let (store, securities) = seeded();
    let parsed = parse_csv(FILE.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    let preview = build_attribute_preview(&parsed, &config, &securities, &[]);
    commit_attributes(&store, &preview).unwrap();

    let defs = store.list_attribute_defs().unwrap();
    let csv = attributes_to_csv(&securities, &defs, &store.security_attributes().unwrap());
    assert!(csv.starts_with("Symbol,ISIN,Name,"));

    let again = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&again);
    let preview = build_attribute_preview(&again, &config, &securities, &defs);
    assert_eq!(preview.new_attributes(), 0);
    assert_eq!(preview.values(), 6);
    assert!(preview.problems.is_empty());
}
