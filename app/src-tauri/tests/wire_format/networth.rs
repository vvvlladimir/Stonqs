use super::*;
use sq_core::calc::{AssetHolding, DebtPayoff, NetWorth, NetWorthPoint, NetWorthSeries};
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
        debt_to_assets: Some(dec!(0.543478)),
        holdings: vec![AssetHolding {
            asset_id: "asset-1".into(),
            name: "Flat".into(),
            kind: AssetKind::Property,
            side: AssetSide::Owned,
            currency: "CHF".into(),
            amount: dec!(380952.38),
            amount_base: dec!(400000),
            valued_on: day(2026, 1, 15),
            days_old: 166,
            stale: false,
            change_base: Some(dec!(15000)),
            changed_since: Some(day(2025, 1, 10)),
            secured_by: None,
            secured_debt_base: Some(dec!(250000)),
            equity_base: Some(dec!(150000)),
            payoff: None,
        }],
        stale_count: 0,
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
            "debt_to_assets",
            "holdings",
            "invested_share",
            "investments_base",
            "net_base",
            "not_valued_yet",
            "owed_base",
            "owned_base",
            "stale_count",
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
            "change_base",
            "changed_since",
            "currency",
            "days_old",
            "equity_base",
            "kind",
            "name",
            "payoff",
            "secured_by",
            "secured_debt_base",
            "side",
            "stale",
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
        days_old: 166,
        stale: false,
        change_base: Some(dec!(-5400)),
        changed_since: Some(day(2025, 1, 10)),
        secured_by: Some("asset-1".into()),
        secured_debt_base: None,
        equity_base: None,
        payoff: Some(DebtPayoff {
            months_left: Some(276),
            payoff_on: Some(day(2049, 6, 30)),
            interest_ahead: Some(dec!(92400)),
            last_payment: Some(dec!(840.55)),
            ends_on: Some(day(2049, 5, 1)),
            paid_share: Some(dec!(0.0736)),
        }),
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

/// A debt's schedule crosses as its own object: months and a date the app worked out, beside the
/// end the contract names, so the two can be shown as the two different things they are.
#[test]
fn a_payoff_carries_the_computed_end_and_the_contracted_one() {
    let mut reading = sample();
    reading.holdings[0].payoff = Some(DebtPayoff {
        months_left: Some(3),
        payoff_on: Some(day(2026, 9, 30)),
        interest_ahead: Some(dec!(18.261)),
        last_payment: Some(dec!(218.261)),
        ends_on: Some(day(2027, 1, 1)),
        paid_share: Some(dec!(0.25)),
    });
    let json = serde_json::to_value(reading).unwrap();
    let payoff = &json["holdings"][0]["payoff"];
    assert_eq!(
        keys(payoff),
        vec![
            "ends_on",
            "interest_ahead",
            "last_payment",
            "months_left",
            "paid_share",
            "payoff_on",
        ]
    );
    assert_eq!(payoff["months_left"], Value::from(3));
    assert_eq!(payoff["interest_ahead"], Value::String("18.261".into()));
    assert_eq!(payoff["payoff_on"], Value::String("2026-09-30".into()));
}

/// A debt with no end reports nulls, not zeros: "never" and "now" are not the same answer.
#[test]
fn a_debt_that_never_ends_reports_null_rather_than_zero() {
    let mut reading = sample();
    reading.holdings[0].payoff = Some(DebtPayoff {
        months_left: None,
        payoff_on: None,
        interest_ahead: None,
        last_payment: None,
        ends_on: None,
        paid_share: None,
    });
    let json = serde_json::to_value(reading).unwrap();
    let payoff = &json["holdings"][0]["payoff"];
    assert_eq!(payoff["months_left"], Value::Null);
    assert_eq!(payoff["interest_ahead"], Value::Null);
    assert_eq!(payoff["payoff_on"], Value::Null);
}

/// Age and staleness cross as a count and a flag; the threshold is the app's, not the screen's.
#[test]
fn the_age_of_a_figure_crosses_as_a_number_of_days() {
    let json = serde_json::to_value(sample()).unwrap();
    assert_eq!(json["holdings"][0]["days_old"], Value::from(166));
    assert_eq!(json["holdings"][0]["stale"], Value::Bool(false));
    assert_eq!(json["stale_count"], Value::from(0));
}
