use super::*;
use crate::calc::{Allocation, AllocationBucket};
use crate::model::{Security, SecurityKind};
use crate::money::normalize_currency;
use rust_decimal_macros::dec;

/// Two securities: 7000/3000 toward a 50/50 target at 10000.
/// Targets are 5000 each: sell 20 at 100 and buy 40 at 50.
#[test]
fn two_security_portfolio_is_brought_to_fifty_fifty() {
    let us = Security::new("VUSA", "US shares", "EUR", SecurityKind::Etf);
    let eu = Security::new("VEUR", "EU shares", "EUR", SecurityKind::Etf);

    let valuation = PortfolioValuation {
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 5).unwrap(),
        base_currency: normalize_currency("EUR"),
        positions: vec![
            position(&us.id, dec!(70), dec!(100), dec!(7000)),
            position(&eu.id, dec!(60), dec!(50), dec!(3000)),
        ],
        securities_value_base: dec!(10000),
        cash_base: Decimal::ZERO,
        total_value_base: dec!(10000),
        cost_basis_base: dec!(10000),
        unrealized_pnl_base: Decimal::ZERO,
        currency_gain_base: Decimal::ZERO,
        realized_pnl_base: Decimal::ZERO,
        realized_currency_gain_base: Decimal::ZERO,
        dividends_base: Decimal::ZERO,
        interest_base: Decimal::ZERO,
        fees_base: Decimal::ZERO,
        taxes_base: Decimal::ZERO,
    };

    let allocation = Allocation {
        total_base: dec!(10000),
        buckets: vec![
            bucket("node-us", dec!(7000), dec!(0.7)),
            bucket("node-eu", dec!(3000), dec!(0.3)),
        ],
    };

    let target = AllocationTarget::new("portfolio", "taxonomy", "50/50")
        .with_weight("node-us", dec!(0.5))
        .with_weight("node-eu", dec!(0.5));

    let assignments = vec![assign(&us.id, "node-us"), assign(&eu.id, "node-eu")];

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-us", "node-eu"]),
        &assignments,
        &[us.clone(), eu.clone()],
        &[],
        RebalanceOptions::default(),
    )
    .unwrap();

    assert_eq!(plan.off_target_base, Decimal::ZERO);
    assert_eq!(plan.items[0].drift_base, dec!(-2000));
    assert_eq!(plan.items[0].trades[0].quantity, dec!(-20));
    assert_eq!(plan.items[1].drift_base, dec!(2000));
    assert_eq!(plan.items[1].trades[0].quantity, dec!(40));

    // 40*50 = 2000 purchases, 20*100 = 2000 sales; both final weights are 0.5.
    assert_eq!(plan.cash_used_base, dec!(2000));
    assert_eq!(plan.sell_base, dec!(2000));
    assert_eq!(plan.cash_left_base, Decimal::ZERO);
    assert_eq!(plan.items[0].trades[0].price_base, dec!(100));
    assert_eq!(plan.items[0].trades[0].weight_after, dec!(0.5));
    assert_eq!(plan.items[1].trades[0].price_base, dec!(50));
    assert_eq!(plan.items[1].trades[0].weight_after, dec!(0.5));
}

/// Buy-only 7000/3000 portfolio with 1000 new cash: Europe receives
/// `2500 * (1000 / 2500) = 1000`, or 20 units at price 50.
#[test]
fn buy_only_mode_spends_no_more_than_the_new_money() {
    let us = Security::new("VUSA", "US shares", "EUR", SecurityKind::Etf);
    let eu = Security::new("VEUR", "EU shares", "EUR", SecurityKind::Etf);

    let valuation = PortfolioValuation {
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 5).unwrap(),
        base_currency: normalize_currency("EUR"),
        positions: vec![
            position(&us.id, dec!(70), dec!(100), dec!(7000)),
            position(&eu.id, dec!(60), dec!(50), dec!(3000)),
        ],
        securities_value_base: dec!(10000),
        cash_base: Decimal::ZERO,
        total_value_base: dec!(10000),
        cost_basis_base: dec!(10000),
        unrealized_pnl_base: Decimal::ZERO,
        currency_gain_base: Decimal::ZERO,
        realized_pnl_base: Decimal::ZERO,
        realized_currency_gain_base: Decimal::ZERO,
        dividends_base: Decimal::ZERO,
        interest_base: Decimal::ZERO,
        fees_base: Decimal::ZERO,
        taxes_base: Decimal::ZERO,
    };
    let allocation = Allocation {
        total_base: dec!(10000),
        buckets: vec![
            bucket("node-us", dec!(7000), dec!(0.7)),
            bucket("node-eu", dec!(3000), dec!(0.3)),
        ],
    };
    let target = AllocationTarget::new("portfolio", "taxonomy", "50/50")
        .with_weight("node-us", dec!(0.5))
        .with_weight("node-eu", dec!(0.5));
    let assignments = vec![assign(&us.id, "node-us"), assign(&eu.id, "node-eu")];

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-us", "node-eu"]),
        &assignments,
        &[us.clone(), eu.clone()],
        &[],
        RebalanceOptions {
            cash_to_invest: dec!(1000),
            allow_sell: false,
        },
    )
    .unwrap();

    assert_eq!(plan.total_base, dec!(11000));
    // The overweight is reported honestly because selling is disabled.
    assert_eq!(plan.items[0].drift_base, dec!(-1500));
    assert!(plan.items[0].trades.is_empty());
    assert_eq!(plan.items[1].drift_base, dec!(2500));
    assert_eq!(plan.items[1].trades[0].quantity, dec!(20));
    assert_eq!(plan.cash_used_base, dec!(1000));

    // No sale and no spending beyond the deposited cash.
    assert!(
        plan.items
            .iter()
            .flat_map(|i| &i.trades)
            .all(|t| t.quantity.is_sign_positive())
    );
    assert!(plan.cash_used_base <= dec!(1000));
    // 1000 deposited, 1000 spent, zero left.
    assert_eq!(plan.sell_base, Decimal::ZERO);
    assert_eq!(plan.cash_left_base, Decimal::ZERO);
}

/// Whole-lot securities floor 250/100 to 2; fractional securities allow 2.5.
#[test]
fn quantity_step_decides_between_two_and_two_and_a_half() {
    let whole = Security::new("WHOLE", "whole lots only", "EUR", SecurityKind::Etf);
    let fractional =
        Security::new("FRAC", "fractional", "EUR", SecurityKind::Etf).with_quantity_step(dec!(0.0001));

    assert_eq!(whole.round_to_step(dec!(2.5)), dec!(2));
    assert_eq!(fractional.round_to_step(dec!(2.5)), dec!(2.5));
}

/// New cash is spent down to tradable steps: 1000 portfolio plus 500 deposit,
/// first pass spends 450 and the leftover 50 buys ten more units at 5.
#[test]
fn new_money_is_spent_down_to_the_tradable_step() {
    let us = Security::new("VUSA", "US shares", "EUR", SecurityKind::Etf);
    let eu = Security::new("VEUR", "EU shares", "EUR", SecurityKind::Etf);

    let valuation = valuation_of(vec![
        position(&us.id, dec!(7), dec!(100), dec!(700)),
        position(&eu.id, dec!(60), dec!(5), dec!(300)),
    ]);
    let allocation = Allocation {
        total_base: dec!(1000),
        buckets: vec![
            bucket("node-us", dec!(700), dec!(0.7)),
            bucket("node-eu", dec!(300), dec!(0.3)),
        ],
    };
    let target = AllocationTarget::new("portfolio", "taxonomy", "50/50")
        .with_weight("node-us", dec!(0.5))
        .with_weight("node-eu", dec!(0.5));

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-us", "node-eu"]),
        &[assign(&us.id, "node-us"), assign(&eu.id, "node-eu")],
        &[us.clone(), eu.clone()],
        &[],
        RebalanceOptions {
            cash_to_invest: dec!(500),
            allow_sell: false,
        },
    )
    .unwrap();

    assert!(
        plan.items[0].trades.is_empty(),
        "one US share costs more than the cash left"
    );
    assert_eq!(plan.items[1].trades[0].quantity, dec!(100));
    assert_eq!(plan.items[1].trades[0].estimated_base, dec!(500));
    assert_eq!(plan.cash_used_base, dec!(500));
    assert_eq!(plan.cash_left_base, Decimal::ZERO);
}

/// Top-up never exceeds available funds: sell 2 at 100 to buy 40 at 5.
#[test]
fn the_top_up_never_spends_money_the_plan_does_not_have() {
    let us = Security::new("VUSA", "US shares", "EUR", SecurityKind::Etf);
    let eu = Security::new("VEUR", "EU shares", "EUR", SecurityKind::Etf);

    let valuation = valuation_of(vec![
        position(&us.id, dec!(7), dec!(100), dec!(700)),
        position(&eu.id, dec!(60), dec!(5), dec!(300)),
    ]);
    let allocation = Allocation {
        total_base: dec!(1000),
        buckets: vec![
            bucket("node-us", dec!(700), dec!(0.7)),
            bucket("node-eu", dec!(300), dec!(0.3)),
        ],
    };
    let target = AllocationTarget::new("portfolio", "taxonomy", "50/50")
        .with_weight("node-us", dec!(0.5))
        .with_weight("node-eu", dec!(0.5));

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-us", "node-eu"]),
        &[assign(&us.id, "node-us"), assign(&eu.id, "node-eu")],
        &[us.clone(), eu.clone()],
        &[],
        RebalanceOptions::default(),
    )
    .unwrap();

    assert_eq!(plan.items[0].trades[0].quantity, dec!(-2));
    assert_eq!(plan.items[1].trades[0].quantity, dec!(40));
    assert_eq!(plan.cash_used_base, dec!(200));
    assert_eq!(plan.sell_base, dec!(200));
    assert_eq!(plan.cash_left_base, Decimal::ZERO);
}

/// A cash target keeps its share as cash, not rounding residue.
/// With 100 new cash, buy three units for 90 and deposit the remaining 10.
#[test]
fn the_cash_node_keeps_its_share_of_the_new_money() {
    let etf = Security::new("VWCE", "world", "EUR", SecurityKind::Etf);
    let cash_subject = "cash:acc-1:EUR";

    let valuation = valuation_of(vec![position(&etf.id, dec!(30), dec!(30), dec!(900))]);
    let allocation = Allocation {
        total_base: dec!(1000),
        buckets: vec![
            bucket("node-equity", dec!(900), dec!(0.9)),
            bucket("node-cash", dec!(100), dec!(0.1)),
        ],
    };
    let target = AllocationTarget::new("portfolio", "taxonomy", "90/10")
        .with_weight("node-equity", dec!(0.9))
        .with_weight("node-cash", dec!(0.1));

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-equity", "node-cash"]),
        &[assign(&etf.id, "node-equity"), assign(cash_subject, "node-cash")],
        std::slice::from_ref(&etf),
        &[cash(cash_subject, "Bank", dec!(100))],
        RebalanceOptions {
            cash_to_invest: dec!(100),
            allow_sell: false,
        },
    )
    .unwrap();

    assert_eq!(plan.items[0].trades[0].quantity, dec!(3));
    assert!(
        plan.items[1].trades.is_empty(),
        "an account balance is not bought"
    );
    // The cash node receives the 10 left after the security purchase.
    assert_eq!(plan.items[1].deposits.len(), 1);
    assert_eq!(plan.items[1].deposits[0].account_id, "acc-1");
    assert_eq!(plan.items[1].deposits[0].currency, "EUR");
    assert_eq!(plan.items[1].deposits[0].current_base, dec!(100));
    assert_eq!(plan.items[1].deposits[0].amount_base, dec!(10));
    // Final cash share: (100 + 10) / 1100 = 0.1.
    assert_eq!(plan.items[1].deposits[0].weight_after, dec!(0.1));
    assert_eq!(plan.cash_used_base, dec!(90));
    assert_eq!(plan.cash_left_base, dec!(10));
}

/// The observed transaction step overrides the ETF default: 50/30 floors
/// to 1.666666 at a 0.000001 step, costing 49.99998.
#[test]
fn the_step_seen_in_the_history_beats_the_default_by_kind() {
    let etf = Security::new("VWCE", "world", "EUR", SecurityKind::Etf);
    let mut position = position(&etf.id, dec!(3.426301), dec!(30), dec!(102.789030));
    position.observed_quantity_step = Some(dec!(0.000001));

    let valuation = valuation_of(vec![position]);
    let allocation = Allocation {
        total_base: dec!(102.789030),
        buckets: vec![bucket("node-equity", dec!(102.789030), Decimal::ONE)],
    };
    let target =
        AllocationTarget::new("portfolio", "taxonomy", "all in one").with_weight("node-equity", Decimal::ONE);

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &roots(&["node-equity"]),
        &[assign(&etf.id, "node-equity")],
        std::slice::from_ref(&etf),
        &[],
        RebalanceOptions {
            cash_to_invest: dec!(50),
            allow_sell: false,
        },
    )
    .unwrap();

    assert_eq!(plan.items[0].trades[0].quantity, dec!(1.666666));
    assert_eq!(plan.items[0].trades[0].estimated_base, dec!(49.99998));
}

fn position(
    security_id: &str,
    quantity: Decimal,
    price: Decimal,
    value: Decimal,
) -> crate::calc::PositionValuation {
    crate::calc::PositionValuation {
        security_id: security_id.to_string(),
        currency: normalize_currency("EUR"),
        cost_currency: normalize_currency("EUR"),
        quantity,
        price,
        fx_rate: Decimal::ONE,
        cost_fx_rate: Decimal::ONE,
        market_value: value,
        market_value_base: value,
        cost_basis: value,
        cost_basis_base: value,
        unrealized_pnl_base: Decimal::ZERO,
        currency_gain_base: Decimal::ZERO,
        realized_pnl_base: Decimal::ZERO,
        observed_quantity_step: None,
    }
}

/// Builds a valuation from ready-made positions for tests.
fn valuation_of(positions: Vec<crate::calc::PositionValuation>) -> PortfolioValuation {
    let securities: Decimal = positions.iter().map(|p| p.market_value_base).sum();
    PortfolioValuation {
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 5).unwrap(),
        base_currency: normalize_currency("EUR"),
        positions,
        securities_value_base: securities,
        cash_base: Decimal::ZERO,
        total_value_base: securities,
        cost_basis_base: securities,
        unrealized_pnl_base: Decimal::ZERO,
        currency_gain_base: Decimal::ZERO,
        realized_pnl_base: Decimal::ZERO,
        realized_currency_gain_base: Decimal::ZERO,
        dividends_base: Decimal::ZERO,
        interest_base: Decimal::ZERO,
        fees_base: Decimal::ZERO,
        taxes_base: Decimal::ZERO,
    }
}

fn bucket(key: &str, value: Decimal, weight: Decimal) -> AllocationBucket {
    AllocationBucket {
        key: key.to_string(),
        label: key.to_string(),
        value_base: value,
        weight,
        children: Vec::new(),
    }
}

/// Builds root nodes with the same IDs used by allocation buckets.
fn roots(ids: &[&str]) -> Vec<TaxonomyNode> {
    ids.iter().map(|id| node(id, None)).collect()
}

/// Builds a cash taxonomy subject with a base-currency balance.
fn cash(key: &str, name: &str, value: Decimal) -> TaxonomySubject {
    TaxonomySubject {
        key: key.to_string(),
        kind: SubjectKind::Cash,
        symbol: "EUR".to_string(),
        name: name.to_string(),
        value_base: value,
        excluded: false,
    }
}

/// Assigns a subject fully to one node, the common test case.
fn assign(subject_id: &str, node_id: &str) -> Assignment {
    Assignment {
        subject_id: subject_id.to_string(),
        node_id: node_id.to_string(),
        weight: Decimal::ONE,
    }
}

fn node(id: &str, parent: Option<&str>) -> TaxonomyNode {
    TaxonomyNode {
        id: id.to_string(),
        taxonomy_id: "taxonomy".to_string(),
        parent_id: parent.map(str::to_string),
        name: id.to_string(),
        rank: 0,
        color: None,
    }
}
