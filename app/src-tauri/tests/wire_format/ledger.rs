use super::*;

/// A trade row flattens the core trade and adds only names, like every other ledger row.
#[test]
fn trade_rows_flatten_the_trade_and_keep_nullable_rates() {
    use sq_app_lib::commands::trades::TradeRow;
    use sq_core::calc::Trade;

    let trade = Trade {
        security_id: "sec-1".into(),
        opened_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        closed_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        quantity: dec!(20),
        entry_value_base: dec!(2210),
        exit_value_base: dec!(2594),
        pnl_base: dec!(384),
        holding_days: 275,
        return_pct: Some(dec!(0.173756)),
        irr: None,
    };
    let json = serde_json::to_value(TradeRow {
        trade,
        symbol: "AAPL".into(),
        name: "Apple".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "closed_at",
            "entry_value_base",
            "exit_value_base",
            "holding_days",
            "irr",
            "name",
            "opened_at",
            "pnl_base",
            "quantity",
            "return_pct",
            "security_id",
            "symbol",
        ]
    );
    // Money and rates are strings; a day count is a number.
    assert_eq!(json["entry_value_base"], Value::String("2210".into()));
    assert_eq!(json["return_pct"], Value::String("0.173756".into()));
    assert!(json["holding_days"].is_number());
    assert!(json["irr"].is_null());
    assert_eq!(json["closed_at"], Value::String("2025-01-01".into()));
}

#[test]
fn corporate_action_rows_flatten_the_action_and_keep_ratios_as_strings() {
    use sq_app_lib::commands::corporate_actions::CorporateActionRow;
    use sq_core::model::{CorporateAction, CorporateActionKind};

    let action = CorporateAction {
        id: "ca-1".into(),
        security_id: "sec-1".into(),
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
        kind: CorporateActionKind::Split,
        ratio_from: dec!(1),
        ratio_to: dec!(4),
        note: None,
    };
    let json = serde_json::to_value(CorporateActionRow {
        action,
        symbol: "NVDA".into(),
        name: "NVIDIA".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "date",
            "id",
            "kind",
            "name",
            "note",
            "ratio_from",
            "ratio_to",
            "security_id",
            "symbol",
        ]
    );
    // A ratio is a Decimal like any other: it crosses as a string, never as a JS number.
    assert_eq!(json["ratio_from"], Value::String("1".into()));
    assert_eq!(json["ratio_to"], Value::String("4".into()));
    assert_eq!(json["kind"], Value::String("SPLIT".into()));
    assert!(json["note"].is_null());
}

#[test]
fn payment_rows_keep_their_axis_as_arrays_of_strings() {
    use sq_app_lib::commands::payments::PayerRow;
    use sq_core::calc::{PaymentLine, PaymentRow, SecurityPaymentRow};

    let line = serde_json::to_value(PaymentRow {
        line: PaymentLine::Dividends,
        amounts: vec![dec!(25), dec!(40)],
        total: dec!(65),
    })
    .unwrap();

    assert_eq!(keys(&line), ["amounts", "line", "total"]);
    assert_eq!(line["line"], Value::String("DIVIDENDS".into()));
    // One amount per bucket, each a string: an axis of JS numbers would be an axis of doubles.
    assert_eq!(
        line["amounts"],
        Value::Array(vec![Value::String("25".into()), Value::String("40".into())])
    );

    let payer = serde_json::to_value(PayerRow {
        row: SecurityPaymentRow {
            security_id: None,
            amounts: vec![dec!(3)],
            total: dec!(3),
        },
        symbol: String::new(),
        name: String::new(),
    })
    .unwrap();

    assert_eq!(
        keys(&payer),
        ["amounts", "name", "security_id", "symbol", "total"]
    );
    // Account interest has no payer, and the frontend writes that row's label itself.
    assert!(payer["security_id"].is_null());
}

/// A plan's schedule crosses as a nested object with its own enum, and a weight is a string
/// like every other number the frontend must not do arithmetic on.
#[test]
fn plan_rows_keep_the_schedule_nested() {
    use sq_app_lib::commands::plans::{PlanLegRow, PlanRow};
    use sq_core::model::{Interval, InvestmentPlan, Schedule};

    let plan = InvestmentPlan::new(
        "pf-1",
        "acc-1",
        "Monthly",
        dec!(500),
        "EUR",
        Schedule::monthly(chrono::NaiveDate::from_ymd_opt(2024, 1, 5).unwrap()),
    )
    .with_leg("sec-1", dec!(60));

    let json = serde_json::to_value(PlanRow {
        legs: vec![PlanLegRow {
            security_id: "sec-1".into(),
            symbol: "IWDA".into(),
            name: "iShares Core MSCI World".into(),
            weight: dec!(60),
            share: dec!(1),
        }],
        plan,
        account_name: "Broker".into(),
        next_date: Some("2024-02-05".into()),
        due_count: 2,
        last_executed: Some("2024-01-05".into()),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "account_name",
            "due_count",
            "last_executed",
            "legs",
            "next_date",
            "plan"
        ]
    );
    assert_eq!(
        keys(&json["plan"]),
        [
            "account_id",
            "active",
            "amount",
            "currency",
            "fees",
            "id",
            "legs",
            "name",
            "note",
            "portfolio_id",
            "schedule",
            "taxes",
        ]
    );
    assert_eq!(keys(&json["plan"]["schedule"]), ["count", "end", "start", "unit"]);
    assert_eq!(json["plan"]["schedule"]["unit"], "MONTH");
    assert_eq!(json["plan"]["amount"], Value::String("500".into()));
    assert_eq!(json["legs"][0]["weight"], Value::String("60".into()));

    // The enum the editor sends back is the one the host reads.
    let unit: Interval = serde_json::from_value(json["plan"]["schedule"]["unit"].clone()).unwrap();
    assert_eq!(unit, Interval::Month);
}

/// An expected dividend crosses flat beside its instrument's name; a date the forecast cannot
/// give and a net it cannot know are null, never an empty string or a zero.
#[test]
fn expected_dividends_keep_unknowns_null() {
    use sq_app_lib::commands::payments::ExpectedDividendRow;
    use sq_core::calc::{DividendFrequency, ExpectedDividend};

    let json = serde_json::to_value(ExpectedDividendRow {
        row: ExpectedDividend {
            security_id: "sec-shell".into(),
            ex_date: chrono::NaiveDate::from_ymd_opt(2025, 5, 3).unwrap(),
            pay_date: None,
            frequency: DividendFrequency::SemiAnnual,
            reported: false,
            per_share: dec!(1.10),
            currency: "EUR".into(),
            quantity: dec!(100),
            gross_base: dec!(110.00),
            net_base: None,
        },
        symbol: "SHEL".into(),
        name: "Shell".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "currency",
            "ex_date",
            "frequency",
            "gross_base",
            "name",
            "net_base",
            "pay_date",
            "per_share",
            "quantity",
            "reported",
            "security_id",
            "symbol"
        ]
    );
    assert_eq!(json["ex_date"], Value::String("2025-05-03".into()));
    assert_eq!(json["frequency"], Value::String("SEMI_ANNUAL".into()));
    assert_eq!(json["gross_base"], Value::String("110.00".into()));
    assert!(json["pay_date"].is_null());
    assert!(json["net_base"].is_null());
}
