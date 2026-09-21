use super::*;

#[test]
fn dashboard_payload_keys_match_the_typescript_types() {
    let json: Value = serde_json::to_value(sample()).unwrap();

    assert_eq!(keys(&json), ["accounts", "cash", "securities", "valuation"]);

    assert_eq!(keys(&json["cash"][0]), ["account_id", "amount", "currency"]);
    assert!(json["cash"][0]["amount"].is_string());
    assert_eq!(keys(&json["accounts"][0]), ["currency", "id", "kind", "name"]);
    assert_eq!(json["accounts"][0]["kind"], "DEPOSIT");

    assert_eq!(
        keys(&json["valuation"]),
        [
            "base_currency",
            "cash_base",
            "cost_basis_base",
            "currency_gain_base",
            "date",
            "dividends_base",
            "fees_base",
            "interest_base",
            "positions",
            "realized_currency_gain_base",
            "realized_pnl_base",
            "securities_value_base",
            "taxes_base",
            "total_value_base",
            "unrealized_pnl_base",
        ]
    );

    assert_eq!(
        keys(&json["valuation"]["positions"][0]),
        [
            "cost_basis",
            "cost_basis_base",
            "cost_currency",
            "cost_fx_rate",
            "currency",
            "currency_gain_base",
            "fx_rate",
            "market_value",
            "market_value_base",
            "observed_quantity_step",
            "price",
            "quantity",
            "realized_pnl_base",
            "security_id",
            "unrealized_pnl_base",
        ]
    );

    assert_eq!(keys(&json["securities"][0]), ["currency", "id", "name", "symbol"]);
}

#[test]
fn money_crosses_the_boundary_as_a_string() {
    let json: Value = serde_json::to_value(sample()).unwrap();

    // Money stays a string at the wire boundary.
    assert_eq!(
        json["valuation"]["total_value_base"],
        Value::String("5672.58".into())
    );
    assert_eq!(
        json["valuation"]["positions"][0]["quantity"],
        Value::String("6".into())
    );
    assert!(json["valuation"]["date"].is_string());
}

/// The FIRE tile: every figure a string, the horizon a plain number, and the currency beside
/// them so the tile does not fetch a valuation of its own just to know what to print.
#[test]
fn a_fire_projection_flattens_under_its_currency() {
    use chrono::NaiveDate;
    use sq_app_lib::commands::plans::FireData;
    use sq_core::calc::{FireAssumptions, fire_projection};

    let projection = fire_projection(
        dec!(200000),
        FireAssumptions {
            annual_spending: dec!(24000),
            withdrawal_rate: dec!(0.04),
            expected_return: dec!(0.05),
            monthly_contribution: dec!(1500),
        },
        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
    )
    .unwrap();
    let json: Value = serde_json::to_value(FireData {
        base_currency: "EUR".into(),
        projection,
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "base_currency",
            "current_base",
            "expected_return",
            "missing_base",
            "monthly_contribution_base",
            "months_to_target",
            "progress",
            "sustainable_annual_base",
            "target_base",
            "target_date",
            "withdrawal_rate"
        ]
    );
    assert_eq!(json["target_base"], "600000");
    assert_eq!(json["sustainable_annual_base"], "8000.00");
    assert_eq!(json["months_to_target"], 132, "a horizon is months, not money");
    assert_eq!(json["target_date"], "2037-01-31");
}
