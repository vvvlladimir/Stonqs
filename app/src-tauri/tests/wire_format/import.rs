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
            "symbol_aliases"
        ]
    );
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
