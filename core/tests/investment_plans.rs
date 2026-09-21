//! Investment plans: what a contribution buys, and what is still due. See ADR-0033.

mod support;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::{
    contribution_schedule, contributions_by_month, due_occurrences, investable_amount, monthly_contribution,
    plan_occurrence, plan_transactions,
};
use sq_core::market::DateRange;
use sq_core::model::{Interval, InvestmentPlan, Schedule, Security, SecurityKind, TransactionKind};
use std::collections::BTreeSet;
use support::{FakePrices, FakeRates, d};

const DEPOT: &str = "acc-depot";
const CASH: &str = "acc-cash";
const WORLD: &str = "sec-world";
const EM: &str = "sec-em";

fn securities() -> Vec<Security> {
    let mut world = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf);
    world.id = WORLD.into();
    let mut em = Security::new("EIMI", "iShares Core MSCI EM IMI", "USD", SecurityKind::Etf);
    em.id = EM.into();
    vec![world, em]
}

fn monthly(amount: Decimal) -> InvestmentPlan {
    InvestmentPlan::new(
        "pf",
        DEPOT,
        "Monthly",
        amount,
        "EUR",
        Schedule::monthly(d(2024, 1, 5)),
    )
}

/// 500 € a month split 60/40 with a 1.50 € flat fee.
/// Investable = 500.00 − 1.50 = 498.50.
/// World leg: 498.50 × 0.6 = 299.10 at 89.50 € → 299.10 / 89.50 = 3.341… → 3 whole units,
///            3 × 89.50 = 268.50, leaving 299.10 − 268.50 = 30.60.
/// EM leg:    498.50 × 0.4 = 199.40, priced 32.00 USD with USD/EUR = 0.90 → 28.80 € a unit,
///            199.40 / 28.80 = 6.923… → 6 units, 6 × 32.00 = 192.00 USD = 172.80 €,
///            leaving 199.40 − 172.80 = 26.60.
/// Fee split: 1.50 × 0.6 = 0.90 and the remainder 0.60.
/// Cash left over the whole occurrence: 500.00 − (268.50 + 0.90) − (172.80 + 0.60) = 57.20.
#[test]
fn a_split_contribution_buys_whole_units_and_keeps_the_remainder() {
    let plan = monthly(dec!(500))
        .with_leg(WORLD, dec!(60))
        .with_leg(EM, dec!(40))
        .with_costs(dec!(1.50), Decimal::ZERO);

    let prices = FakePrices::new("EUR")
        .with(WORLD, d(2024, 3, 5), dec!(89.50))
        .with_in(EM, d(2024, 3, 5), dec!(32.00), "USD");
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 3, 5), dec!(0.90));

    let occurrence = plan_occurrence(&plan, d(2024, 3, 5), &securities(), &prices, &rates).unwrap();

    let world = &occurrence.trades[0];
    assert_eq!(world.budget, dec!(299.10));
    assert_eq!(world.quantity, dec!(3));
    assert_eq!(world.amount, dec!(268.50));
    assert_eq!(world.fees, dec!(0.90));
    assert_eq!(world.cash_left, dec!(30.60));

    let em = &occurrence.trades[1];
    assert_eq!(em.budget, dec!(199.40));
    assert_eq!(em.quantity, dec!(6));
    assert_eq!(em.amount, dec!(192.00)); // quoted in USD
    assert_eq!(em.currency, "USD");
    assert_eq!(em.fx_rate, dec!(0.90));
    assert_eq!(em.fees, dec!(0.60));
    assert_eq!(em.cash_left, dec!(26.60));

    assert_eq!(occurrence.cash_left, dec!(57.20));
}

/// The drafts are ordinary buys: quantity and price come from the occurrence, the flat cost
/// rides along on the leg it was charged to, and the amount is `quantity × price` in the
/// listing's currency — 3 × 89.50 = 268.50 EUR.
#[test]
fn drafts_are_ordinary_buy_transactions() {
    let plan = monthly(dec!(500))
        .with_leg(WORLD, dec!(1))
        .with_costs(dec!(1.50), Decimal::ZERO);
    let prices = FakePrices::new("EUR").with(WORLD, d(2024, 3, 5), dec!(89.50));
    let rates = FakeRates::new();

    let occurrence = plan_occurrence(&plan, d(2024, 3, 5), &securities(), &prices, &rates).unwrap();
    let drafts = plan_transactions(&plan, &occurrence);

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].kind, TransactionKind::Buy);
    assert_eq!(drafts[0].account_id, DEPOT);
    assert_eq!(drafts[0].security_id.as_deref(), Some(WORLD));
    assert_eq!(drafts[0].quantity, dec!(5)); // 498.50 / 89.50 = 5.569… -> 5
    assert_eq!(drafts[0].amount, dec!(447.50)); // 5 × 89.50
    assert_eq!(drafts[0].fees, dec!(1.50));
}

/// A contribution too small for one unit is a real answer: 50 € against a 89.50 € share buys
/// nothing, the money stays cash, and no zero-quantity row is written.
#[test]
fn a_contribution_below_one_unit_writes_nothing() {
    let plan = monthly(dec!(50)).with_leg(WORLD, dec!(1));
    let prices = FakePrices::new("EUR").with(WORLD, d(2024, 3, 5), dec!(89.50));
    let rates = FakeRates::new();

    let occurrence = plan_occurrence(&plan, d(2024, 3, 5), &securities(), &prices, &rates).unwrap();
    assert!(occurrence.buys_nothing());
    assert_eq!(occurrence.cash_left, dec!(50));
    assert!(plan_transactions(&plan, &occurrence).is_empty());
}

/// A plan with no legs moves money and buys nothing: one Deposit for the full amount.
#[test]
fn a_cash_plan_drafts_a_deposit() {
    let plan = InvestmentPlan::new(
        "pf",
        CASH,
        "Salary",
        dec!(1000),
        "EUR",
        Schedule::monthly(d(2024, 1, 1)),
    );
    let prices = FakePrices::new("EUR");
    let rates = FakeRates::new();

    let occurrence = plan_occurrence(&plan, d(2024, 3, 1), &securities(), &prices, &rates).unwrap();
    let drafts = plan_transactions(&plan, &occurrence);

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].kind, TransactionKind::Deposit);
    assert_eq!(drafts[0].account_id, CASH);
    assert_eq!(drafts[0].amount, dec!(1000));
    assert_eq!(investable_amount(&plan), dec!(1000));
}

/// Occurrences from 5 Jan to 5 Apr are four; February was committed, so three are still due.
#[test]
fn committed_occurrences_drop_out_of_the_due_list() {
    let plan = monthly(dec!(500)).with_leg(WORLD, dec!(1));
    let executed = BTreeSet::from([d(2024, 2, 5)]);

    let due = due_occurrences(&plan, &executed, d(2024, 4, 5)).unwrap();
    assert_eq!(due, vec![d(2024, 1, 5), d(2024, 3, 5), d(2024, 4, 5)]);
}

/// Stopping a plan must not leave a backlog waiting behind the switch.
#[test]
fn an_inactive_plan_is_due_for_nothing() {
    let mut plan = monthly(dec!(500)).with_leg(WORLD, dec!(1));
    plan.active = false;
    assert!(
        due_occurrences(&plan, &BTreeSet::new(), d(2024, 4, 5))
            .unwrap()
            .is_empty()
    );
}

/// Two plans, one in euro and one in dollars, projected over the second quarter of 2024.
/// The euro plan pays 500 on 5 Apr / 5 May / 5 Jun. The dollar plan pays 300 USD quarterly
/// from 15 Jan, so inside the window only 15 Apr falls, converted at the rate known on the
/// reporting date: 300 × 0.90 = 270.00 €.
/// April therefore totals 500 + 270 = 770.00 €.
#[test]
fn the_projection_converts_at_todays_rate() {
    let euro = monthly(dec!(500)).with_leg(WORLD, dec!(1));
    let dollar = InvestmentPlan::new(
        "pf",
        DEPOT,
        "Quarterly USD",
        dec!(300),
        "USD",
        Schedule::monthly(d(2024, 1, 15)).every(3, Interval::Month),
    )
    .with_leg(EM, dec!(1));
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 3, 31), dec!(0.90));

    let range = DateRange::new(d(2024, 4, 1), d(2024, 6, 30));
    let plans = vec![euro, dollar];
    let schedule = contribution_schedule(&plans, range, "EUR", &rates, d(2024, 3, 31)).unwrap();

    assert_eq!(schedule.len(), 4);
    assert_eq!(schedule[0].date, d(2024, 4, 5));
    assert_eq!(schedule[1].date, d(2024, 4, 15));
    assert_eq!(schedule[1].amount_base, dec!(270.00));

    let by_month = contributions_by_month(&schedule);
    assert_eq!(by_month["2024-04"], dec!(770.00));
    assert_eq!(by_month["2024-05"], dec!(500.00));
    assert_eq!(by_month["2024-06"], dec!(500.00));
}

/// A leg with no quote on the occurrence date is missing data, not a zero-unit purchase.
#[test]
fn a_leg_without_a_price_is_missing_market_data() {
    let plan = monthly(dec!(500)).with_leg(WORLD, dec!(1));
    let prices = FakePrices::new("EUR");
    let rates = FakeRates::new();

    let err = plan_occurrence(&plan, d(2024, 3, 5), &securities(), &prices, &rates).unwrap_err();
    assert!(matches!(err, sq_core::error::Error::MissingMarketData { .. }));
}

/// A monthly 500 € plan and a quarterly 300 USD one, read over the next twelve months:
/// 12 × 500 = 6000 € plus 4 × 300 × 0.90 = 1080 €, so 7080 / 12 = 590.00 € a month.
#[test]
fn the_monthly_rate_is_read_off_a_year_not_off_the_interval() {
    let euro = monthly(dec!(500)).with_leg(WORLD, dec!(1));
    let dollar = InvestmentPlan::new(
        "pf",
        DEPOT,
        "Quarterly USD",
        dec!(300),
        "USD",
        Schedule::monthly(d(2024, 1, 15)).every(3, Interval::Month),
    )
    .with_leg(EM, dec!(1));
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 1, 1), dec!(0.90));

    let rate = monthly_contribution(&[euro, dollar], "EUR", &rates, d(2024, 1, 1)).unwrap();
    assert_eq!(rate, dec!(590.00));
}
