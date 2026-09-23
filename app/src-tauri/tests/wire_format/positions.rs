use super::*;

/// Flattened fields must retain their frontend-facing names.
#[test]
fn flattened_rows_keep_both_halves() {
    use sq_app_lib::commands::accounts::{AccountRow, CashBalance};
    use sq_app_lib::commands::transactions::TransactionRow;
    use sq_core::model::{Account, Transaction};

    let cash = Account::deposit("Bank", "EUR");
    let account = Account::securities("Broker", "EUR", &cash.id);
    let json = serde_json::to_value(AccountRow {
        account: account.clone(),
        transaction_count: 3,
        in_portfolio: true,
        reference_name: Some(cash.name.clone()),
        balances: vec![CashBalance {
            currency: "EUR".into(),
            amount: dec!(102.15),
        }],
        securities_value_base: Some(dec!(2449.44)),
        value_base: Some(dec!(2551.59)),
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "balances",
            "currency",
            "id",
            "in_portfolio",
            "is_active",
            "kind",
            "name",
            "opened_at",
            "reference_account_id",
            "reference_name",
            "securities_value_base",
            "transaction_count",
            "value_base"
        ]
    );
    // Enum values use the core's SCREAMING_SNAKE_CASE representation.
    assert_eq!(json["kind"], "SECURITIES");
    assert_eq!(json["balances"][0]["amount"], "102.15");
    assert_eq!(json["securities_value_base"], "2449.44");
    assert_eq!(json["value_base"], "2551.59");

    let day = chrono::NaiveDate::from_ymd_opt(2024, 6, 5).unwrap();
    let json = serde_json::to_value(TransactionRow {
        transaction: Transaction::buy(&account.id, "sec-1", day, dec!(5), dec!(195), "USD"),
        symbol: Some("AAPL".into()),
        account_name: "Broker".into(),
        amount_base: dec!(926.25),
        net_base: dec!(-926.25),
    })
    .unwrap();
    assert_eq!(
        keys(&json),
        [
            "account_id",
            "account_name",
            "amount",
            "amount_base",
            "currency",
            "date",
            "external_id",
            "fee_currency",
            "fees",
            "fx_rate_to_base",
            "id",
            "kind",
            "link_id",
            "net_base",
            "note",
            "price",
            "quantity",
            "security_id",
            "symbol",
            "tax_currency",
            "taxes"
        ]
    );
    assert_eq!(json["kind"], "BUY");
    assert_eq!(json["amount"], Value::String("975".into()));
}

/// Position-return fields must preserve their nullable wire shape.
#[test]
fn position_return_row_keys_match_the_typescript_types() {
    use sq_app_lib::commands::positions::PositionReturnRow;

    let row = PositionReturnRow {
        security_id: "sec-1".into(),
        symbol: "IWDA.L".into(),
        name: "iShares Core MSCI World".into(),
        twr: Some(dec!(0.261765)),
        twr_annualized: Some(dec!(0.261765)),
        xirr: None,
        pnl_base: dec!(350),
        absolute_performance: Some(dec!(0.269061)),
        fees_base: dec!(9.90),
        taxes_base: dec!(0),
        contribution: dec!(0.116667),
        risk: Some(sq_core::calc::PositionRisk {
            volatility: 0.18,
            semi_deviation: 0.12,
            max_drawdown: -0.2,
            max_drawdown_days: None,
        }),
    };
    let json: Value = serde_json::to_value(row).unwrap();

    assert_eq!(
        keys(&json),
        [
            "absolute_performance",
            "contribution",
            "fees_base",
            "name",
            "pnl_base",
            "risk",
            "security_id",
            "symbol",
            "taxes_base",
            "twr",
            "twr_annualized",
            "xirr",
        ]
    );
    assert_eq!(json["twr"], "0.261765");
    assert!(json["xirr"].is_null());
    assert_eq!(json["pnl_base"], "350");
    // Statistics, not money: numbers, like the portfolio's own risk metrics.
    assert_eq!(
        keys(&json["risk"]),
        [
            "max_drawdown",
            "max_drawdown_days",
            "semi_deviation",
            "volatility"
        ]
    );
    assert_eq!(json["risk"]["volatility"], 0.18);
    assert!(json["risk"]["max_drawdown_days"].is_null());
}

/// Position rows include nullable daily changes and monetary strings.
#[test]
fn position_row_carries_the_day_change() {
    use rust_decimal::Decimal;
    use sq_app_lib::commands::positions::PositionRow;
    use sq_core::calc::DividendFrequency;

    let row = |previous: Option<Decimal>| PositionRow {
        security_id: "sec-1".into(),
        symbol: "IWDA.L".into(),
        name: "iShares Core MSCI World".into(),
        currency: "USD".into(),
        cost_currency: "EUR".into(),
        quantity: dec!(6),
        price: dec!(250),
        fx_rate: dec!(0.95),
        market_value_base: dec!(1425),
        cost_basis_base: dec!(1163.30),
        unrealized_pnl_base: dec!(261.70),
        currency_gain_base: dec!(11.30),
        instrument_gain_base: dec!(250.40),
        realized_pnl_base: dec!(97.70),
        dividends_base: dec!(9.66),
        dividend_frequency: DividendFrequency::Quarterly,
        dividend_payments: 4,
        dividend_last: Some("2024-12-15".into()),
        dividend_year_base: dec!(9.66),
        dividend_yield: Some(dec!(0.006779)),
        yield_on_cost: Some(dec!(0.008304)),
        ath_price: Some(dec!(262)),
        ath_date: Some("2024-11-29".into()),
        ath_distance: Some(dec!(-0.045802)),
        weight: dec!(0.25),
        previous_price: previous,
        day_change_base: previous.map(|_| dec!(11.40)),
        day_change: previous.map(|_| dec!(0.008064)),
        accounts: vec![],
        lots: vec![],
    };

    let json: Value = serde_json::to_value(row(Some(dec!(248)))).unwrap();
    assert_eq!(
        keys(&json),
        [
            "accounts",
            "ath_date",
            "ath_distance",
            "ath_price",
            "cost_basis_base",
            "cost_currency",
            "currency",
            "currency_gain_base",
            "day_change",
            "day_change_base",
            "dividend_frequency",
            "dividend_last",
            "dividend_payments",
            "dividend_year_base",
            "dividend_yield",
            "dividends_base",
            "fx_rate",
            "instrument_gain_base",
            "lots",
            "market_value_base",
            "name",
            "previous_price",
            "price",
            "quantity",
            "realized_pnl_base",
            "security_id",
            "symbol",
            "unrealized_pnl_base",
            "weight",
            "yield_on_cost",
        ]
    );
    assert_eq!(json["day_change_base"], "11.40");
    assert_eq!(json["previous_price"], "248");

    let undefined: Value = serde_json::to_value(row(None)).unwrap();
    assert!(undefined["previous_price"].is_null());
    assert!(undefined["day_change_base"].is_null());
    assert!(undefined["day_change"].is_null());
}

/// The cost-basis comparison keeps one object per method, so a column reads `fifo.cost_basis`
/// rather than a prefixed field: the two are the same shape and must stay comparable.
#[test]
fn a_cost_basis_row_keeps_one_object_per_method() {
    use rust_decimal::Decimal;
    use sq_app_lib::commands::positions::PositionCostRow;
    use sq_core::calc::{CostBasisFigures, CostBasisRow};

    let figures = |cost: Decimal, unrealized: Decimal| CostBasisFigures {
        cost_basis: cost,
        cost_basis_base: cost,
        cost_per_unit: cost / dec!(10),
        cost_per_unit_base: cost / dec!(10),
        unrealized_pnl_base: unrealized,
        realized_pnl_base: dec!(1000),
    };
    let json: Value = serde_json::to_value(PositionCostRow {
        security_id: "sec-1".into(),
        symbol: "AAPL".into(),
        name: "Apple Inc.".into(),
        row: CostBasisRow {
            security_id: "sec-1".into(),
            cost_currency: "EUR".into(),
            quantity: dec!(10),
            market_value_base: dec!(1800),
            fifo: figures(dec!(1500), dec!(300)),
            average: figures(dec!(1250), dec!(550)),
        },
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "average",
            "cost_currency",
            "fifo",
            "market_value_base",
            "name",
            "quantity",
            "security_id",
            "symbol"
        ]
    );
    assert_eq!(
        keys(&json["fifo"]),
        [
            "cost_basis",
            "cost_basis_base",
            "cost_per_unit",
            "cost_per_unit_base",
            "realized_pnl_base",
            "unrealized_pnl_base"
        ]
    );
    assert_eq!(json["fifo"]["cost_per_unit"], "150");
    assert_eq!(json["average"]["cost_basis_base"], "1250");
    assert_eq!(json["average"]["unrealized_pnl_base"], "550");
}
