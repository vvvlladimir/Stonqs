use super::*;
use sq_core::calc::{AssetHolding, NetWorth, NetWorthPoint, NetWorthSeries};
use sq_core::model::{AssetKind, AssetSide};

fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn sample() -> NetWorth {
    NetWorth {
        date: day(2026, 6, 30),
        base_currency: "EUR".into(),
        investments_base: dec!(30000),
        owned_base: dec!(400000),
        owed_base: dec!(250000),
        net_base: dec!(180000),
        invested_share: Some(dec!(0.166667)),
        holdings: vec![AssetHolding {
            asset_id: "asset-1".into(),
            name: "Flat".into(),
            kind: AssetKind::Property,
            side: AssetSide::Owned,
            currency: "CHF".into(),
            amount: dec!(380952.38),
            amount_base: dec!(400000),
            valued_on: day(2026, 1, 15),
            secured_by: None,
        }],
        not_valued_yet: vec!["asset-2".into()],
    }
}

/// Every figure crosses as a string, and the three parts cross beside the sum: the frontend
/// stacks them rather than subtracting anything.
#[test]
fn a_net_worth_reading_crosses_as_strings_with_its_parts() {
    let json = serde_json::to_value(sample()).unwrap();
    assert_eq!(
        keys(&json),
        vec![
            "base_currency",
            "date",
            "holdings",
            "invested_share",
            "investments_base",
            "net_base",
            "not_valued_yet",
            "owed_base",
            "owned_base",
        ]
    );
    assert_eq!(json["investments_base"], Value::String("30000".into()));
    assert_eq!(json["owed_base"], Value::String("250000".into()));
    assert_eq!(json["net_base"], Value::String("180000".into()));
    assert_eq!(json["date"], Value::String("2026-06-30".into()));
}

/// A holding says what it is, which way it points, and the day its figure is from — the reading
/// date is not that day, and the screen shows both.
#[test]
fn a_holding_carries_its_kind_side_and_valuation_day() {
    let json = serde_json::to_value(sample()).unwrap();
    let holding = &json["holdings"][0];
    assert_eq!(
        keys(holding),
        vec![
            "amount",
            "amount_base",
            "asset_id",
            "currency",
            "kind",
            "name",
            "secured_by",
            "side",
            "valued_on",
        ]
    );
    assert_eq!(holding["kind"], Value::String("PROPERTY".into()));
    assert_eq!(holding["side"], Value::String("OWNED".into()));
    assert_eq!(holding["currency"], Value::String("CHF".into()));
    assert_eq!(holding["amount"], Value::String("380952.38".into()));
    assert_eq!(holding["valued_on"], Value::String("2026-01-15".into()));
}

/// A debt is a kind, not a minus sign: the amount stays positive on the wire.
#[test]
fn a_debt_crosses_positive_under_the_owed_side() {
    let mut reading = sample();
    reading.holdings[0] = AssetHolding {
        asset_id: "debt".into(),
        name: "Mortgage".into(),
        kind: AssetKind::Mortgage,
        side: AssetSide::Owed,
        currency: "EUR".into(),
        amount: dec!(250000),
        amount_base: dec!(250000),
        valued_on: day(2026, 1, 15),
        secured_by: Some("asset-1".into()),
    };
    let json = serde_json::to_value(reading).unwrap();
    assert_eq!(json["holdings"][0]["side"], Value::String("OWED".into()));
    assert_eq!(json["holdings"][0]["amount"], Value::String("250000".into()));
    assert_eq!(json["holdings"][0]["secured_by"], Value::String("asset-1".into()));
}

/// Zero or negative net worth has no share invested, and `null` is not `"0"`.
#[test]
fn an_unanswerable_share_is_null() {
    let mut reading = sample();
    reading.invested_share = None;
    let json = serde_json::to_value(reading).unwrap();
    assert_eq!(json["invested_share"], Value::Null);
}

#[test]
fn a_line_point_carries_the_day_and_the_three_parts() {
    let series = NetWorthSeries {
        base_currency: "EUR".into(),
        points: vec![NetWorthPoint {
            date: day(2026, 1, 15),
            investments_base: dec!(30000),
            owned_base: dec!(400000),
            owed_base: dec!(250000),
            net_base: dec!(180000),
        }],
    };
    let json = serde_json::to_value(series).unwrap();
    assert_eq!(keys(&json), vec!["base_currency", "points"]);
    assert_eq!(
        keys(&json["points"][0]),
        vec!["date", "investments_base", "net_base", "owed_base", "owned_base"]
    );
    assert_eq!(json["points"][0]["date"], Value::String("2026-01-15".into()));
    assert_eq!(json["points"][0]["owned_base"], Value::String("400000".into()));
}
