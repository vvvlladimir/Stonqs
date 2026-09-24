use super::*;

/// Import enums and mapping keys must keep their wire names.
#[test]
fn import_enums_cross_as_screaming_snake_case() {
    use sq_core::import::{
        AmountSign, ImportField, ImportMapping, ImportProblem, ProblemCode, RowStatus, Severity,
    };

    assert_eq!(
        serde_json::to_value(RowStatus::UnknownSecurity).unwrap(),
        Value::String("UNKNOWN_SECURITY".into())
    );
    assert_eq!(
        serde_json::to_value(RowStatus::Ignored).unwrap(),
        Value::String("IGNORED".into())
    );
    assert_eq!(
        serde_json::to_value(RowStatus::Similar).unwrap(),
        Value::String("SIMILAR".into())
    );
    assert_eq!(
        serde_json::to_value(ImportField::FxRate).unwrap(),
        Value::String("FX_RATE".into())
    );
    assert_eq!(
        serde_json::to_value(AmountSign::Signed).unwrap(),
        Value::String("SIGNED".into())
    );

    // The frontend groups and colors issues by these fields.
    let problem = ImportProblem::row(ProblemCode::DirectionFromSign, 7, "…").warn();
    let json = serde_json::to_value(&problem).unwrap();
    assert_eq!(json["code"], Value::String("DIRECTION_FROM_SIGN".into()));
    assert_eq!(json["severity"], Value::String("WARNING".into()));
    assert_eq!(keys(&json), ["code", "column", "message", "row", "severity"]);
    assert_eq!(
        serde_json::to_value(Severity::Error).unwrap(),
        Value::String("ERROR".into())
    );

    let mapping = ImportMapping::default().with_column(ImportField::Date, "Datum");
    let json = serde_json::to_value(&mapping).unwrap();
    assert_eq!(json["columns"]["DATE"], Value::String("Datum".into()));
    assert_eq!(
        keys(&json),
        [
            "account_aliases",
            "account_id",
            "amount_basis",
            "amount_sign",
            "columns",
            "default_currency",
            "ignored_kinds",
            "kind_aliases",
            "new_securities",
            "rules",
            "symbol_aliases"
        ]
    );
}

/// Every count the commit screen shows is its own field, and a row status the screen switches
/// on has to reach it under the name the label table uses.
#[test]
fn the_import_summary_counts_each_reason_separately() {
    use sq_core::import::{ImportOptions, ImportResult, ImportSummary};

    let json = serde_json::to_value(ImportSummary::default()).unwrap();
    assert_eq!(
        keys(&json),
        [
            "duplicates",
            "ignored",
            "invalid",
            "ready",
            "similar",
            "total",
            "unknown_securities",
            "updated",
            "warnings"
        ]
    );

    // A row only a stored operation resembles is skipped *and* counted apart, so the screen can
    // offer to write it after all instead of leaving the user to guess what "skipped" held.
    let json = serde_json::to_value(ImportResult::default()).unwrap();
    assert_eq!(
        keys(&json),
        [
            "created_securities",
            "imported",
            "problems",
            "similar",
            "skipped",
            "updated"
        ]
    );

    let json = serde_json::to_value(ImportOptions::default()).unwrap();
    assert_eq!(json["import_similar"], Value::Bool(false));
    assert_eq!(json["import_duplicates"], Value::Bool(false));
}

/// A suggested pair carries both ids, both accounts by name, and the two amounts as strings —
/// the screen states what it is about to join before the user joins it.
#[test]
fn a_transfer_suggestion_names_both_legs() {
    use sq_app_lib::commands::transactions::TransferSuggestion;
    use sq_core::calc::TransferPair;

    let json = serde_json::to_value(TransferSuggestion {
        pair: TransferPair {
            out_id: "t-1".into(),
            in_id: "t-2".into(),
            currency: "EUR".into(),
            amount_out: dec!(1000),
            amount_in: dec!(998),
            date_out: "2024-06-03".into(),
            date_in: "2024-06-05".into(),
            account_out: "acc-a".into(),
            account_in: "acc-b".into(),
            days_apart: 2,
        },
        account_out_name: "Trade Republic · cash".into(),
        account_in_name: "IBKR · cash".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "account_in",
            "account_in_name",
            "account_out",
            "account_out_name",
            "amount_in",
            "amount_out",
            "currency",
            "date_in",
            "date_out",
            "days_apart",
            "in_id",
            "out_id"
        ]
    );
    assert_eq!(json["amount_out"], "1000");
}

/// Resolved security and search candidates are hand-mapped in TypeScript.
#[test]
fn resolved_securities_cross_with_stable_field_names() {
    use sq_core::import::SecurityDraft;
    use sq_core::market::SecurityMatch;

    let found = SecurityMatch {
        source: "yahoo".into(),
        symbol: "CSSPX.MI".into(),
        name: "iShares Core S&P 500 UCITS ETF USD (Acc)".into(),
        exchange: Some("Milan".into()),
        mic: Some("XMIL".into()),
        kind: sq_core::model::SecurityKind::Etf,
        currency: Some("EUR".into()),
        isin: Some("IE00B5BMR087".into()),
        has_history: Some(true),
        last_close: Some(dec!(715.30)),
    };
    let json = serde_json::to_value(&found).unwrap();
    assert_eq!(
        keys(&json),
        [
            "currency",
            "exchange",
            "has_history",
            "isin",
            "kind",
            "last_close",
            "mic",
            "name",
            "source",
            "symbol"
        ]
    );
    assert_eq!(json["last_close"], "715.30");
    assert_eq!(json["kind"], "ETF");

    let json = serde_json::to_value(SecurityDraft::from_match(&found, "EUR")).unwrap();
    assert_eq!(
        keys(&json),
        [
            "currency",
            "data_source",
            "data_symbol",
            "exchange",
            "isin",
            "kind",
            "mic",
            "name",
            "symbol"
        ]
    );
    assert_eq!(json["data_source"], "yahoo");
}

/// Taxonomy import preview is consumed directly by the frontend.
#[test]
fn taxonomy_import_preview_keys_match_the_typescript_types() {
    use sq_core::import::{ParseConfig, build_taxonomy_preview, detect_taxonomy_config, parse_csv};

    let csv = "Levels 1,Levels 2,Levels 3,Weight,Allocation,Symbol,ISIN\n\
Regions,United States,,,60.00,,\n\
Regions,United States,Apple Inc.,72.60,,AAPL,US0378331005\n";
    let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_taxonomy_config(&parsed);
    let security =
        sq_core::model::Security::new("AAPL", "Apple Inc.", "USD", sq_core::model::SecurityKind::Stock);
    let preview = build_taxonomy_preview(&parsed, &config, std::slice::from_ref(&security), None);
    let json: Value = serde_json::to_value(&preview).unwrap();

    assert_eq!(
        keys(&json),
        ["assignments", "config", "kind", "name", "nodes", "problems"]
    );
    assert_eq!(
        keys(&json["config"]),
        ["isin", "levels", "root_is_name", "symbol", "target", "weight"]
    );
    assert_eq!(keys(&json["nodes"][0]), ["path", "target"]);
    assert_eq!(
        keys(&json["assignments"][0]),
        [
            "isin",
            "label",
            "matched_by",
            "path",
            "row",
            "security_id",
            "symbol",
            "weight",
        ]
    );

    assert_eq!(json["assignments"][0]["weight"], Value::String("0.7260".into()));
    assert_eq!(json["nodes"][0]["target"], Value::String("0.60".into()));
    assert_eq!(json["kind"], "REGION");
}

/// The attribute preview crosses as the plan it is: columns with their kind, rows with the
/// values they would receive, and the problems that stopped the rest.
#[test]
fn attribute_import_preview_keys_match_the_typescript_types() {
    use sq_core::import::{ParseConfig, build_attribute_preview, detect_attribute_config, parse_csv};

    let csv = "Symbol,ISIN,Name,TER,Domicile\nAAPL,US0378331005,Apple Inc.,0.00,United States\n";
    let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_attribute_config(&parsed);
    let mut security =
        sq_core::model::Security::new("AAPL", "Apple Inc.", "USD", sq_core::model::SecurityKind::Stock);
    security.isin = Some("US0378331005".into());
    let preview = build_attribute_preview(&parsed, &config, std::slice::from_ref(&security), &[]);
    let json: Value = serde_json::to_value(&preview).unwrap();

    assert_eq!(keys(&json), ["attributes", "config", "problems", "rows"]);
    assert_eq!(keys(&json["config"]), ["attributes", "isin", "name", "symbol"]);
    assert_eq!(
        keys(&json["attributes"][0]),
        ["attribute_id", "kind", "name", "values"]
    );
    // A kind crosses as the core's own spelling, so the picker and the database agree.
    assert_eq!(json["attributes"][0]["kind"], Value::String("NUMBER".into()));
    assert_eq!(
        keys(&json["rows"][0]),
        [
            "isin",
            "label",
            "matched_by",
            "row",
            "security_id",
            "symbol",
            "values"
        ]
    );
    // Values are keyed by the attribute's name: an id nobody has seen yet cannot be one.
    assert_eq!(
        json["rows"][0]["values"]["Domicile"],
        Value::String("United States".into())
    );
    assert_eq!(json["rows"][0]["matched_by"], Value::String("isin".into()));
}

/// What a plugin's reader adds to the wizard's payload, and nothing else: the preview itself is
/// unchanged, because a reader produces the app's own transaction file and stops there (ADR-0073).
#[test]
fn a_reader_reaches_the_wizard_as_an_id_and_its_own_warnings() {
    use sq_app_lib::plugins::reader::ReaderWarning;

    let json = serde_json::to_value(ReaderWarning {
        plugin: "app.stonqs.mt940/mt940".into(),
        row: Some(3),
        code: "unreadable-line".into(),
        message: "a statement line was skipped".into(),
    })
    .unwrap();

    assert_eq!(keys(&json), ["code", "message", "plugin", "row"]);
    assert_eq!(json["row"], 3);
}
