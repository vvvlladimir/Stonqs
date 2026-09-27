use super::*;
use rust_decimal_macros::dec;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

#[test]
fn monthly_plan_keeps_the_day_of_month() {
    // Start 31 Jan 2024 (a leap year): Feb has 29 days, Mar has 31. Measuring every
    // occurrence from the start rather than the previous one keeps the 31st.
    let s = Schedule::monthly(d("2024-01-31"));
    assert_eq!(s.nth(0).unwrap(), d("2024-01-31"));
    assert_eq!(s.nth(1).unwrap(), d("2024-02-29"));
    assert_eq!(s.nth(2).unwrap(), d("2024-03-31"));
    assert_eq!(s.nth(3).unwrap(), d("2024-04-30"));
}

#[test]
fn quarterly_is_three_months() {
    let s = Schedule::monthly(d("2024-01-15")).every(3, Interval::Month);
    assert_eq!(s.nth(1).unwrap(), d("2024-04-15"));
    assert_eq!(s.nth(4).unwrap(), d("2025-01-15"));
}

#[test]
fn weekly_steps_seven_days() {
    let s = Schedule {
        start: d("2024-06-03"),
        end: None,
        unit: Interval::Week,
        count: 2,
    };
    assert_eq!(s.nth(1).unwrap(), d("2024-06-17"));
}

#[test]
fn occurrences_are_clipped_by_the_window_and_the_end() {
    let s = Schedule::monthly(d("2024-01-10")).until(d("2024-05-01"));
    // Occurrences: 10 Jan, 10 Feb, 10 Mar, 10 Apr; the plan ends 1 May, the window starts 1 Feb.
    let got = s.occurrences(d("2024-02-01"), d("2024-12-31")).unwrap();
    assert_eq!(got, vec![d("2024-02-10"), d("2024-03-10"), d("2024-04-10")]);
}

#[test]
fn next_after_stops_at_the_end() {
    let s = Schedule::monthly(d("2024-01-10")).until(d("2024-03-01"));
    assert_eq!(s.next_after(d("2024-01-10")).unwrap(), Some(d("2024-02-10")));
    assert_eq!(s.next_after(d("2024-02-10")).unwrap(), None);
}

#[test]
fn leg_amount_divides_by_the_sum_of_weights() {
    // 500 split 6 / 4 must equal 500 split 0.6 / 0.4: 500 * 6 / 10 = 300.
    let plan = InvestmentPlan::new(
        "p",
        "acc",
        "Monthly",
        dec!(500),
        "EUR",
        Schedule::monthly(d("2024-01-01")),
    )
    .with_leg("a", dec!(6))
    .with_leg("b", dec!(4));
    assert_eq!(plan.leg_amount(&plan.legs[0]).unwrap(), dec!(300));
    assert_eq!(plan.leg_amount(&plan.legs[1]).unwrap(), dec!(200));
}

#[test]
fn a_plan_cannot_name_one_instrument_twice() {
    let plan = InvestmentPlan::new(
        "p",
        "acc",
        "Monthly",
        dec!(500),
        "EUR",
        Schedule::monthly(d("2024-01-01")),
    )
    .with_leg("a", dec!(1))
    .with_leg("a", dec!(1));
    assert!(plan.validate().is_err());
}

#[test]
fn costs_may_not_swallow_the_contribution() {
    let plan = InvestmentPlan::new(
        "p",
        "acc",
        "Monthly",
        dec!(10),
        "EUR",
        Schedule::monthly(d("2024-01-01")),
    )
    .with_leg("a", dec!(1))
    .with_costs(dec!(10), Decimal::ZERO);
    assert!(plan.validate().is_err());
}

#[test]
fn a_cash_plan_has_no_legs_and_still_validates() {
    let plan = InvestmentPlan::new(
        "p",
        "cash",
        "Salary",
        dec!(1000),
        "EUR",
        Schedule::monthly(d("2024-01-01")),
    );
    assert!(plan.is_cash_only());
    plan.validate().unwrap();
}
