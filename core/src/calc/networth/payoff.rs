//! What a debt still costs and when it ends, read forward from the balance the owner last wrote.
//!
//! The schedule never restates the balance (ADR-0092): it takes today's figure as given and asks
//! what happens next if nothing changes. Worked month by month in `Decimal` rather than through
//! the closed-form annuity formula, because the interest it reports has to be the sum of the
//! months actually paid — a logarithm would give the count and not the money.

use crate::model::Amortization;
use chrono::{Months, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Longer than this is not a plan, and a date fifty years out pretends to a precision the
/// assumptions do not have.
const MAX_MONTHS: u32 = 600;

/// Where a debt is going, on the owner's own assumptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtPayoff {
    /// Whole months until the balance is gone. `None` when the payment does not cover the
    /// month's interest — the debt grows, and no honest date exists — or when it would take
    /// longer than fifty years.
    pub months_left: Option<u32>,
    /// The month `months_left` lands on, counted from the reading date.
    pub payoff_on: Option<NaiveDate>,
    /// Interest still to pay if nothing changes: the sum of what each remaining month accrues.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub interest_ahead: Option<Decimal>,
    /// What the last payment is, which is a part payment unless the schedule divides exactly.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub last_payment: Option<Decimal>,
    /// The end the schedule itself names. Shown beside `payoff_on`: the two disagreeing is
    /// information, not an error.
    pub ends_on: Option<NaiveDate>,
    /// How much of the debt is gone, measured from the first figure ever written for it to
    /// today's. `None` when there is only one figure, or the first one was zero.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub paid_share: Option<Decimal>,
}

/// `balance` and `schedule` are in the debt's own currency; the answer is too.
pub fn payoff(
    balance: Decimal,
    first_balance: Option<Decimal>,
    schedule: &Amortization,
    date: NaiveDate,
) -> DebtPayoff {
    let run = amortize(balance, schedule);
    DebtPayoff {
        months_left: run.as_ref().map(|r| r.months),
        payoff_on: run
            .as_ref()
            .and_then(|r| date.checked_add_months(Months::new(r.months))),
        interest_ahead: run.as_ref().map(|r| r.interest),
        last_payment: run.as_ref().map(|r| r.last_payment),
        ends_on: schedule.ends_on,
        paid_share: paid_share(balance, first_balance),
    }
}

struct Run {
    months: u32,
    interest: Decimal,
    last_payment: Decimal,
}

/// Monthly rate is the yearly one divided by twelve — the convention a monthly payment implies,
/// and the one a lender's own statement uses.
fn amortize(balance: Decimal, schedule: &Amortization) -> Option<Run> {
    let payment = schedule.monthly_payment;
    if payment <= Decimal::ZERO || balance <= Decimal::ZERO {
        return None;
    }
    let monthly = schedule.rate / Decimal::from(12);

    let mut left = balance;
    let mut interest = Decimal::ZERO;
    for month in 1..=MAX_MONTHS {
        let accrued = left * monthly;
        // The payment has to beat the month's interest, or the balance never falls.
        if payment <= accrued {
            return None;
        }
        interest += accrued;
        let due = left + accrued;
        if due <= payment {
            return Some(Run {
                months: month,
                interest,
                last_payment: due,
            });
        }
        left = due - payment;
    }
    None
}

/// From the first figure ever written to today's. Above one is impossible and below zero means
/// the debt grew, which is a real answer and stays negative.
fn paid_share(balance: Decimal, first: Option<Decimal>) -> Option<Decimal> {
    let first = first?;
    if first <= Decimal::ZERO {
        return None;
    }
    Some((first - balance) / first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn schedule(rate: Decimal, payment: Decimal) -> Amortization {
        Amortization {
            rate,
            monthly_payment: payment,
            ends_on: None,
        }
    }

    /// 1 000 owed, no interest, 250 a month: four months, the last one exactly 250, no interest.
    #[test]
    fn a_debt_with_no_interest_is_the_balance_over_the_payment() {
        let answer = payoff(
            dec!(1000),
            None,
            &schedule(Decimal::ZERO, dec!(250)),
            day(2026, 1, 31),
        );

        assert_eq!(answer.months_left, Some(4));
        assert_eq!(answer.interest_ahead, Some(Decimal::ZERO));
        assert_eq!(answer.last_payment, Some(dec!(250)));
        assert_eq!(answer.payoff_on, Some(day(2026, 5, 31)));
    }

    /// 1 000 at 12% a year is 1% a month, paying 400:
    ///   month 1: interest 10.00, due 1 010.00, left 610.00
    ///   month 2: interest  6.10, due   616.10, left 216.10
    ///   month 3: interest  2.161, due 218.261 — under 400, so the last payment is 218.261
    /// Three months, interest 10 + 6.10 + 2.161 = 18.261.
    #[test]
    fn interest_is_the_sum_of_the_months_actually_paid() {
        let answer = payoff(
            dec!(1000),
            None,
            &schedule(dec!(0.12), dec!(400)),
            day(2026, 1, 31),
        );

        assert_eq!(answer.months_left, Some(3));
        assert_eq!(answer.interest_ahead, Some(dec!(18.261)));
        assert_eq!(answer.last_payment, Some(dec!(218.261)));
    }

    /// 200 000 at 6% is 1 000 of interest in the first month. A payment of 1 000 pays that and
    /// nothing else, for ever — so there is no date, rather than a date fifty years out.
    #[test]
    fn a_payment_that_only_covers_the_interest_has_no_end() {
        let answer = payoff(
            dec!(200000),
            None,
            &schedule(dec!(0.06), dec!(1000)),
            day(2026, 1, 31),
        );

        assert_eq!(answer.months_left, None);
        assert_eq!(answer.payoff_on, None);
        assert_eq!(answer.interest_ahead, None);
    }

    /// The schedule's own end is carried through untouched, so a payoff that lands elsewhere can
    /// be compared with what the contract says.
    #[test]
    fn the_schedules_own_end_is_reported_beside_the_computed_one() {
        let mut plan = schedule(dec!(0.0345), dec!(1100));
        plan.ends_on = Some(day(2049, 5, 1));
        let answer = payoff(dec!(211200), None, &plan, day(2026, 1, 31));

        assert_eq!(answer.ends_on, Some(day(2049, 5, 1)));
        assert!(answer.months_left.is_some_and(|m| m > 240), "{answer:?}");
    }

    /// 228 000 owed at the first figure, 211 200 now: 16 800 gone, which is 7.368…% of it.
    #[test]
    fn progress_is_measured_from_the_first_figure_ever_written() {
        let answer = payoff(
            dec!(211200),
            Some(dec!(228000)),
            &schedule(dec!(0.0345), dec!(1100)),
            day(2026, 1, 31),
        );

        assert_eq!(
            answer.paid_share.map(|s| s.round_dp(6)),
            Some((dec!(16800) / dec!(228000)).round_dp(6))
        );
    }

    /// A debt that grew since the first figure reads as negative progress, not as zero.
    #[test]
    fn a_debt_that_grew_reports_negative_progress() {
        let answer = payoff(
            dec!(5200),
            Some(dec!(4000)),
            &schedule(Decimal::ZERO, dec!(200)),
            day(2026, 1, 31),
        );

        assert_eq!(answer.paid_share, Some(dec!(-0.3)));
    }
}
