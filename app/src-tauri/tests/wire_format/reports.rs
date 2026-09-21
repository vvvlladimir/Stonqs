use super::*;

/// Reports flatten the year and summary into one object.
#[test]
fn report_rows_keep_year_and_summary_together() {
    use sq_app_lib::commands::reports::YearGains;
    use sq_core::calc::RealizedSummary;

    let summary = RealizedSummary {
        disposals: 2,
        proceeds_base: dec!(816.96),
        cost_base: dec!(718.33),
        fees_base: dec!(0.92),
        taxes_base: dec!(0),
        gain_base: dec!(97.70),
        currency_gain_base: dec!(12.30),
    };
    let json = serde_json::to_value(YearGains {
        year: 2024,
        return_on_cost: summary.return_on_cost(),
        summary,
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "cost_base",
            "currency_gain_base",
            "disposals",
            "fees_base",
            "gain_base",
            "proceeds_base",
            "return_on_cost",
            "taxes_base",
            "year"
        ]
    );
    assert!(json["year"].is_number());
    assert_eq!(json["gain_base"], Value::String("97.70".into()));
    // A rate crosses as a string like any other decimal, and stays null when there is no cost.
    assert_eq!(
        json["return_on_cost"],
        Value::String("0.1360099118789414336029401529".into())
    );
}

/// A ledger row flattens its core record and adds only names: the UI reads one flat object.
#[test]
fn report_ledger_rows_flatten_their_record() {
    use sq_app_lib::commands::reports::{ChargeRow, DisposalRow};
    use sq_core::calc::{ChargeRecord, RealizedGain};
    use sq_core::model::{Lot, TransactionKind};

    let gain = RealizedGain {
        date: chrono::NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
        security_id: "sec-1".into(),
        kind: TransactionKind::Sell,
        quantity: dec!(4),
        proceeds_base: dec!(480),
        fees_base: dec!(3),
        taxes_base: dec!(2),
        cost_base: dec!(402),
        cost_in_currency: dec!(402),
        gain_base: dec!(73),
        currency_gain_base: dec!(0),
        lots: vec![Lot {
            acquired_at: chrono::NaiveDate::from_ymd_opt(2023, 9, 1).unwrap(),
            quantity: dec!(4),
            cost_per_unit: dec!(100.50),
            cost_per_unit_base: dec!(100.50),
        }],
    };
    let json = serde_json::to_value(DisposalRow {
        return_on_cost: gain.return_on_cost(),
        gain,
        symbol: "AAPL".into(),
        name: "Apple".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "cost_base",
            "cost_in_currency",
            "currency_gain_base",
            "date",
            "fees_base",
            "gain_base",
            "kind",
            "lots",
            "name",
            "proceeds_base",
            "quantity",
            "return_on_cost",
            "security_id",
            "symbol",
            "taxes_base"
        ]
    );
    assert_eq!(json["kind"], Value::String("SELL".into()));
    assert_eq!(json["quantity"], Value::String("4".into()));

    let json = serde_json::to_value(ChargeRow {
        record: ChargeRecord {
            date: chrono::NaiveDate::from_ymd_opt(2024, 8, 1).unwrap(),
            account_id: "acc-1".into(),
            security_id: None,
            kind: TransactionKind::Fee,
            amount_base: dec!(12),
            currency: "EUR".into(),
            amount_in_currency: dec!(12),
        },
        symbol: String::new(),
        name: String::new(),
        account: "Broker".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "account",
            "account_id",
            "amount_base",
            "amount_in_currency",
            "currency",
            "date",
            "kind",
            "name",
            "security_id",
            "symbol"
        ]
    );
    // An account-level charge keeps a null security rather than an empty string.
    assert_eq!(json["security_id"], Value::Null);
}

/// Income events flatten the core record and keep enum values as strings.
#[test]
fn income_events_keep_the_record_fields_flat() {
    use sq_app_lib::commands::reports::{IncomeEvent, KindIncome};
    use sq_core::calc::{IncomeRecord, IncomeSummary};
    use sq_core::model::TransactionKind;

    let event = IncomeEvent {
        record: IncomeRecord {
            date: chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            account_id: "acc-1".into(),
            security_id: Some("sec-1".into()),
            kind: TransactionKind::Dividend,
            gross_base: dec!(27.00),
            taxes_base: dec!(4.05),
            fees_base: dec!(0),
            net_base: dec!(22.95),
            currency: "USD".into(),
            gross_in_currency: dec!(30),
        },
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
    };
    let json: Value = serde_json::to_value(event).unwrap();

    assert_eq!(
        keys(&json),
        [
            "account_id",
            "currency",
            "date",
            "fees_base",
            "gross_base",
            "gross_in_currency",
            "kind",
            "name",
            "net_base",
            "security_id",
            "symbol",
            "taxes_base",
        ]
    );
    assert_eq!(json["kind"], "DIVIDEND");
    assert_eq!(json["date"], "2024-03-15");
    assert_eq!(json["net_base"], "22.95");
    assert_eq!(json["gross_in_currency"], "30");

    // Account interest has no security.
    let interest: Value = serde_json::to_value(IncomeEvent {
        record: IncomeRecord {
            date: chrono::NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
            account_id: "acc-1".into(),
            security_id: None,
            kind: TransactionKind::Interest,
            gross_base: dec!(10),
            taxes_base: dec!(0),
            fees_base: dec!(0),
            net_base: dec!(10),
            currency: "EUR".into(),
            gross_in_currency: dec!(10),
        },
        symbol: String::new(),
        name: String::new(),
    })
    .unwrap();
    assert!(interest["security_id"].is_null());

    let kind = KindIncome {
        kind: TransactionKind::InterestCharge,
        summary: IncomeSummary {
            events: 1,
            gross_base: dec!(-4),
            taxes_base: dec!(0),
            fees_base: dec!(0),
            net_base: dec!(-4),
        },
    };
    let json: Value = serde_json::to_value(kind).unwrap();
    assert_eq!(
        keys(&json),
        [
            "events",
            "fees_base",
            "gross_base",
            "kind",
            "net_base",
            "taxes_base"
        ]
    );
    assert_eq!(json["kind"], "INTEREST_CHARGE");
    assert_eq!(json["net_base"], "-4");
}

/// Calendar footer and year composition flatten the same summary as every other rollup.
#[test]
fn income_rollups_flatten_the_summary() {
    use sq_app_lib::commands::reports::{MonthOfYearIncome, YearKindIncome};
    use sq_core::calc::IncomeSummary;
    use sq_core::model::TransactionKind;

    let summary = || IncomeSummary {
        events: 2,
        gross_base: dec!(40),
        taxes_base: dec!(0),
        fees_base: dec!(0),
        net_base: dec!(40),
    };

    let json: Value = serde_json::to_value(MonthOfYearIncome {
        month: 6,
        summary: summary(),
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "events",
            "fees_base",
            "gross_base",
            "month",
            "net_base",
            "taxes_base"
        ]
    );
    assert_eq!(json["month"], 6);
    assert_eq!(json["net_base"], "40");

    let json: Value = serde_json::to_value(YearKindIncome {
        year: 2025,
        kind: TransactionKind::Dividend,
        summary: summary(),
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "events",
            "fees_base",
            "gross_base",
            "kind",
            "net_base",
            "taxes_base",
            "year"
        ]
    );
    assert_eq!(json["kind"], "DIVIDEND");
    assert_eq!(json["year"], 2025);
}

/// The taxonomy split keeps its summary nested and its nodes recursive: a node is read as a
/// tree, not as a row, so the summary stays one object rather than merging into the node.
#[test]
fn income_by_taxonomy_nests_the_summary_under_each_node() {
    use sq_app_lib::commands::reports::TaxonomyIncomeData;
    use sq_core::calc::{IncomeNode, IncomeSummary, TaxonomyIncome};

    let summary = |net: rust_decimal::Decimal| IncomeSummary {
        events: 1,
        gross_base: net,
        taxes_base: dec!(0),
        fees_base: dec!(0),
        net_base: net,
    };
    let json: Value = serde_json::to_value(TaxonomyIncomeData {
        taxonomy_id: "tax-1".into(),
        base_currency: "EUR".into(),
        income: TaxonomyIncome {
            total: summary(dec!(85)),
            nodes: vec![IncomeNode {
                key: "node-1".into(),
                label: "Equity".into(),
                summary: summary(dec!(85)),
                weight: dec!(1),
                children: vec![IncomeNode {
                    key: "node-2".into(),
                    label: "US".into(),
                    summary: summary(dec!(51)),
                    weight: dec!(0.6),
                    children: vec![],
                }],
            }],
        },
    })
    .unwrap();

    assert_eq!(keys(&json), ["base_currency", "nodes", "taxonomy_id", "total"]);
    assert_eq!(
        keys(&json["nodes"][0]),
        ["children", "key", "label", "summary", "weight"]
    );
    assert_eq!(json["total"]["net_base"], "85");
    assert_eq!(json["nodes"][0]["weight"], "1");
    assert_eq!(json["nodes"][0]["children"][0]["summary"]["net_base"], "51");
    assert_eq!(json["nodes"][0]["children"][0]["summary"]["events"], 1);
}
