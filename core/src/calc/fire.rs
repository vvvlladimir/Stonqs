//! How far the portfolio is from paying for the year, and how long the current pace would take.
//!
//! Two questions, one arithmetic. The target is what a portfolio must be worth for a yearly
//! withdrawal to be sustainable at a chosen rate — the "4% rule" is a withdrawal rate of `0.04`
//! and therefore a target of twenty-five years' spending. The horizon is when saving at the
//! current monthly rate, compounded at an assumed return, first reaches it.
//!
//! Everything here is an *assumption* the user typed, not a measurement: the expected return is
//! not this portfolio's past return and is never read from it, because the question is "what
//! would it take", not "what happened". The only measured inputs are today's value and what the
//! active plans add up to in a month.

use crate::error::{Error, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};

/// The longest horizon worth naming. Past this, a date is arithmetic rather than a plan.
const MAX_MONTHS: u32 = 1200;

/// The assumptions a projection is made under; all of them the user's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireAssumptions {
    /// What the portfolio must cover in a year, in base currency.
    pub annual_spending: Decimal,
    /// Share of the capital withdrawn per year; `0.04` is the rule of thumb.
    pub withdrawal_rate: Decimal,
    /// Assumed yearly return from here on; `0.05` means 5%. Subtract inflation yourself to read
    /// the answer in today's money — the calculation does not know which one it was given.
    pub expected_return: Decimal,
    /// Paid in every month from here on, in base currency.
    pub monthly_contribution: Decimal,
}

/// Where the portfolio stands against the target, and when the pace would reach it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireProjection {
    /// Capital the yearly spending needs: `annual_spending / withdrawal_rate`.
    #[serde(with = "rust_decimal::serde::str")]
    pub target_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub current_base: Decimal,
    /// What is still missing; zero once the target is met.
    #[serde(with = "rust_decimal::serde::str")]
    pub missing_base: Decimal,
    /// `current / target`; above one when the target is already passed.
    #[serde(with = "rust_decimal::serde::str")]
    pub progress: Decimal,
    /// What today's capital would sustain in a year at the same withdrawal rate.
    #[serde(with = "rust_decimal::serde::str")]
    pub sustainable_annual_base: Decimal,
    /// Months until the target is first reached. `None` when this pace does not get there
    /// within a hundred years — including when it never does.
    pub months_to_target: Option<u32>,
    /// The month `months_to_target` lands on; `None` for the same reason.
    pub target_date: Option<NaiveDate>,
    /// Echoed back so a tile can say what it assumed without keeping its own copy.
    #[serde(with = "rust_decimal::serde::str")]
    pub monthly_contribution_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub expected_return: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub withdrawal_rate: Decimal,
}

/// Projects `current_base` forward under `assumptions`.
///
/// A withdrawal rate of zero has no target to speak of and is refused; a negative one, or
/// negative spending, is not a question this answers.
pub fn fire_projection(
    current_base: Decimal,
    assumptions: FireAssumptions,
    as_of: NaiveDate,
) -> Result<FireProjection> {
    let FireAssumptions {
        annual_spending,
        withdrawal_rate,
        expected_return,
        monthly_contribution,
    } = assumptions;
    if withdrawal_rate <= Decimal::ZERO {
        return Err(Error::Invalid("withdrawal rate must be above zero".into()));
    }
    if annual_spending < Decimal::ZERO {
        return Err(Error::Invalid("annual spending cannot be negative".into()));
    }

    let target = annual_spending / withdrawal_rate;
    let months = months_to_reach(current_base, target, monthly_contribution, expected_return);
    let target_date = months.and_then(|n| as_of.checked_add_months(chrono::Months::new(n)));

    Ok(FireProjection {
        missing_base: (target - current_base).max(Decimal::ZERO),
        progress: if target.is_zero() {
            Decimal::ZERO
        } else {
            current_base / target
        },
        sustainable_annual_base: current_base * withdrawal_rate,
        months_to_target: months,
        target_date,
        target_base: target,
        current_base,
        monthly_contribution_base: monthly_contribution,
        expected_return,
        withdrawal_rate,
    })
}

/// Months of saving `contribution` into `current`, compounded monthly at `yearly_return`, before
/// it first reaches `target`.
///
/// `f64`: this is a horizon read off a logarithm, not money. The result is a whole month either
/// way, so the precision `Decimal` would add lands well below what the assumptions are worth.
fn months_to_reach(
    current: Decimal,
    target: Decimal,
    contribution: Decimal,
    yearly_return: Decimal,
) -> Option<u32> {
    if current >= target {
        return Some(0);
    }
    let (value, goal, paid) = (to_f64(current)?, to_f64(target)?, to_f64(contribution)?);
    let yearly = to_f64(yearly_return)?;
    // A yearly rate compounded monthly, so twelve months of it make exactly the year.
    if yearly <= -1.0 {
        return None;
    }
    let monthly = (1.0 + yearly).powf(1.0 / 12.0) - 1.0;

    let months = if monthly.abs() < 1e-12 {
        // No growth: only the contributions close the gap.
        if paid <= 0.0 {
            return None;
        }
        (goal - value) / paid
    } else {
        // value·(1+i)^n + paid·((1+i)^n − 1)/i = goal, solved for n.
        let from = value * monthly + paid;
        let to = goal * monthly + paid;
        // Both sides must sit on the same side of the standstill balance −paid/i; otherwise the
        // path turns away from the target rather than towards it.
        if from <= 0.0 || to <= 0.0 || to / from <= 0.0 {
            return None;
        }
        (to / from).ln() / (1.0 + monthly).ln()
    };

    if !months.is_finite() || months < 0.0 {
        return None;
    }
    // A partial month has not reached the target yet, so the answer is the month that has.
    let months = months.ceil();
    if months > f64::from(MAX_MONTHS) {
        return None;
    }
    Some(months as u32)
}

fn to_f64(value: Decimal) -> Option<f64> {
    value.to_f64().filter(|v| v.is_finite())
}

/// Reads a rate a user typed, in percent, as the fraction the calculation works in.
pub fn percent_to_rate(percent: Decimal) -> Option<Decimal> {
    Decimal::from_f64(100.0).map(|hundred| percent / hundred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Nothing grows and nothing is paid in: the gap never closes.
    #[test]
    fn a_standstill_reaches_nothing() {
        let projection = fire_projection(
            dec!(100000),
            FireAssumptions {
                annual_spending: dec!(24000),
                withdrawal_rate: dec!(0.04),
                expected_return: Decimal::ZERO,
                monthly_contribution: Decimal::ZERO,
            },
            d(2026, 1, 31),
        )
        .unwrap();

        assert_eq!(projection.target_base, dec!(600000));
        assert_eq!(projection.months_to_target, None);
        assert_eq!(projection.target_date, None);
    }

    /// A withdrawal rate of zero asks for infinite capital, which is not an answer.
    #[test]
    fn a_zero_withdrawal_rate_is_refused() {
        let result = fire_projection(
            dec!(1),
            FireAssumptions {
                annual_spending: dec!(1),
                withdrawal_rate: Decimal::ZERO,
                expected_return: dec!(0.05),
                monthly_contribution: dec!(1),
            },
            d(2026, 1, 31),
        );
        assert!(matches!(result, Err(Error::Invalid(_))));
    }
}
