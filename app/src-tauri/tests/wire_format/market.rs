use super::*;

/// Sparkline quotes keep date, price, and currency together.
#[test]
fn quote_keys() {
    let json: Value = serde_json::to_value(sq_core::market::Quote {
        security_id: "sec-1".into(),
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 3).unwrap(),
        close: dec!(101.5),
        currency: "USD".into(),
        source: "yahoo".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        ["close", "currency", "date", "security_id", "source"]
    );
    assert_eq!(json["date"], "2024-06-03");
    assert_eq!(json["close"], "101.5");
}

/// The instrument's own facts: flattened security fields plus the attribute map.
#[test]
fn security_rows_carry_the_note_wkn_and_attribute_values() {
    use sq_app_lib::commands::securities::SecurityRow;
    use sq_core::model::{AttributeKind, Security, SecurityAttributeDef, SecurityKind};

    let mut security = Security::new("IWDA.L", "iShares Core MSCI World", "USD", SecurityKind::Etf);
    security.wkn = Some("A0RPWH".into());
    security.note = Some("Core holding".into());
    let row = SecurityRow {
        security,
        transaction_count: 3,
        effective_quantity_step: dec!(1),
        quantity_step_observed: None,
        needs_lookup: false,
        quote_currency: Some("USD".into()),
        venue: Some("London Stock Exchange".into()),
        quote_count: 250,
        coverage_from: Some("2024-01-01".into()),
        coverage_to: Some("2024-12-31".into()),
        last_close: Some(dec!(96.42)),
        attributes: [("attr-1".to_string(), "0.2".to_string())].into_iter().collect(),
        other_symbols: [("eodhd".to_string(), "IWDA.LSE".to_string())]
            .into_iter()
            .collect(),
        sparse_history: false,
    };
    let json: Value = serde_json::to_value(row).unwrap();

    // The security is flattened, so its own fields sit beside the directory's.
    assert_eq!(json["wkn"], "A0RPWH");
    assert_eq!(json["note"], "Core holding");
    // Values are keyed by attribute id: renaming an attribute must not orphan them.
    assert_eq!(json["attributes"]["attr-1"], "0.2");
    // Fallback symbols are keyed by source id.
    assert_eq!(json["other_symbols"]["eodhd"], "IWDA.LSE");
    // A verdict, not a count: the table paints the history cell from it.
    assert_eq!(json["sparse_history"], false);

    let def = SecurityAttributeDef::new("TER", AttributeKind::Number).with_unit("%");
    let json: Value = serde_json::to_value(def).unwrap();
    assert_eq!(keys(&json), ["id", "kind", "name", "position", "unit"]);
    assert_eq!(json["kind"], "NUMBER");
}

/// A trigger travels with where the price stands and its latest crossings; a log line carries
/// its rule's kind and instrument. Levels and prices are strings, a problem is a code (ADR-0034).
#[test]
fn alert_rows_carry_the_trigger_its_standing_and_its_log() {
    use chrono::NaiveDate;
    use sq_app_lib::commands::alerts::{AlertProblem, AlertRow, CrossingRow, SecurityEventRow};
    use sq_core::calc::AlertStatus;
    use sq_core::model::{
        AlertCrossing, AlertKind, AlertSide, CrossingDirection, SecurityAlert, SecurityEvent,
    };

    let day = |d| NaiveDate::from_ymd_opt(2024, 6, d).unwrap();
    let alert = SecurityAlert::price("sec-1", dec!(180), "EUR", day(1));
    let crossing = AlertCrossing::new(&alert, day(4), CrossingDirection::Up, Some(dec!(181.35)));
    let json = serde_json::to_value(AlertRow {
        alert: alert.clone(),
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
        status: Some(AlertStatus {
            price: Some(dec!(186.00)),
            price_date: Some(day(5)),
            side: Some(AlertSide::Above),
            distance: Some(dec!(-0.032258)),
        }),
        problem: None,
        crossings: vec![crossing.clone()],
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        ["alert", "crossings", "name", "problem", "status", "symbol"]
    );
    assert_eq!(
        keys(&json["alert"]),
        [
            "checked_through",
            "created_on",
            "currency",
            "date",
            "direction",
            "id",
            "kind",
            "note",
            "price",
            "security_id",
            "side"
        ]
    );
    assert_eq!(json["alert"]["kind"], "PRICE");
    assert_eq!(json["alert"]["price"], Value::String("180".into()));
    assert_eq!(keys(&json["status"]), ["distance", "price", "price_date", "side"]);
    assert_eq!(json["status"]["distance"], Value::String("-0.032258".into()));
    assert_eq!(json["status"]["side"], "ABOVE");
    assert_eq!(
        serde_json::to_value(AlertProblem::MissingRate).unwrap(),
        "missing_rate"
    );

    let json = serde_json::to_value(CrossingRow {
        crossing,
        kind: AlertKind::Price,
        note: None,
        security_id: "sec-1".into(),
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "alert_id",
            "currency",
            "date",
            "direction",
            "id",
            "kind",
            "level",
            "name",
            "note",
            "notified",
            "price",
            "security_id",
            "seen",
            "symbol",
        ]
    );
    assert_eq!(json["direction"], "UP");
    assert_eq!(json["level"], Value::String("180".into()));
    assert_eq!(json["price"], Value::String("181.35".into()));

    let json = serde_json::to_value(SecurityEventRow {
        event: SecurityEvent::split("sec-1", day(10), dec!(1), dec!(10), "yahoo"),
        symbol: "NVDA".into(),
        name: "NVIDIA".into(),
        recorded: false,
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "amount",
            "currency",
            "date",
            "id",
            "kind",
            "name",
            "note",
            "ratio_from",
            "ratio_to",
            "recorded",
            "security_id",
            "source",
            "symbol",
        ]
    );
    assert_eq!(json["kind"], "SPLIT");
    assert_eq!(json["ratio_to"], Value::String("10".into()));
}

/// A watch row is the instrument, its quote figures flat beside it, and the nearest level
/// nested. Prices and fractions are strings; an absent figure is `null` (ADR-0035).
#[test]
fn watch_rows_flatten_the_quote_figures() {
    use chrono::NaiveDate;
    use sq_app_lib::commands::watchlists::WatchRow;
    use sq_core::calc::{InstrumentMove, NearestLevel};
    use sq_core::model::{AlertDirection, SecurityKind};

    let json = serde_json::to_value(WatchRow {
        security_id: "sec-1".into(),
        symbol: "SAP".into(),
        name: "SAP SE".into(),
        kind: SecurityKind::Stock,
        quote: InstrumentMove {
            currency: Some("EUR".into()),
            price: Some(dec!(57)),
            price_date: NaiveDate::from_ymd_opt(2024, 6, 6),
            period_return: Some(dec!(0.14)),
            ..InstrumentMove::default()
        },
        nearest_level: Some(NearestLevel {
            alert_id: "al-1".into(),
            level: dec!(55.29),
            currency: "EUR".into(),
            direction: AlertDirection::Both,
            distance: dec!(-0.03),
        }),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "ath_date",
            "ath_distance",
            "ath_price",
            "currency",
            "day_change",
            "dividend_last",
            "dividend_year",
            "dividend_yield",
            "high",
            "kind",
            "low",
            "name",
            "nearest_level",
            "period_return",
            "period_start",
            "previous_price",
            "price",
            "price_date",
            "range_position",
            "security_id",
            "start_price",
            "symbol",
        ]
    );
    assert_eq!(json["price"], Value::String("57".into()));
    assert_eq!(json["price_date"], "2024-06-06");
    assert_eq!(json["day_change"], Value::Null);
    assert_eq!(
        keys(&json["nearest_level"]),
        ["alert_id", "currency", "direction", "distance", "level"]
    );
    assert_eq!(json["nearest_level"]["distance"], Value::String("-0.03".into()));
}

/// A source row names its roles and its key rule as codes; the frontend writes the words.
#[test]
fn market_source_row_keys() {
    let json: Value = serde_json::to_value(sq_app_lib::commands::sources::MarketSourceRow {
        id: "twelvedata",
        site: "https://twelvedata.com",
        capabilities: vec!["quotes", "fx_rates"],
        key: "required",
        has_key: false,
        on_by_default: true,
        wanted: true,
        active: false,
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "active",
            "capabilities",
            "has_key",
            "id",
            "key",
            "on_by_default",
            "site",
            "wanted"
        ]
    );
    assert_eq!(json["capabilities"][1], "fx_rates");
}

/// A custom source crosses as plain data; the format is tagged by `kind`.
#[test]
fn custom_source_keys() {
    let json: Value = serde_json::to_value(sq_core::market::CustomSource {
        id: "custom:bank".into(),
        label: "Bank".into(),
        role: sq_core::market::CustomRole::Quotes,
        url: "https://bank.example/{SYMBOL}".into(),
        headers: vec![],
        format: sq_core::market::CustomFormat::Csv {
            date_column: "Date".into(),
            close_column: "Close".into(),
        },
        date_format: None,
        factor: Some(dec!(0.01)),
        currency: None,
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "currency",
            "date_format",
            "factor",
            "format",
            "headers",
            "id",
            "label",
            "role",
            "url"
        ]
    );
    assert_eq!(json["format"]["kind"], "csv");
    assert_eq!(json["role"], "quotes");
    assert_eq!(json["factor"], "0.01");
}
