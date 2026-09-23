//! A savings goal: how far a named set of accounts is from an amount, and what it would take.
//!
//! Everything here is the user's intention (ADR-0068). The expected return is an assumption they
//! typed and is never read from what the portfolio actually returned — the same rule FIRE follows
//! (ADR-0059) — and the annuity solved is the one `fire` already solves, so the two cannot
//! disagree about what compounding means.

use super::fire::{monthly_needed, months_needed};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::Goal;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Where a goal stands, and what closing it would take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal_id: String,
    pub name: String,
    /// The target in base currency, converted at the reading date's rate.
    #[serde(with = "rust_decimal::serde::str")]
    pub target_base: Decimal,
    /// What the goal's accounts are worth — the whole portfolio when it names none.
    #[serde(with = "rust_decimal::serde::str")]
    pub current_base: Decimal,
    /// What is still missing; zero once the target is met.
    #[serde(with = "rust_decimal::serde::str")]
    pub missing_base: Decimal,
    /// `current / target`; above one when the target is already passed.
    #[serde(with = "rust_decimal::serde::str")]
    pub progress: Decimal,
    /// Whole months from the reading date to the target date; `None` without one, negative
    /// never — a date already past leaves zero months, not a negative deadline.
    pub months_left: Option<u32>,
    /// With a target date: what must be paid in monthly to arrive on time. `None` when the goal
    /// names no date, and zero when the target is already met.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub required_monthly_base: Option<Decimal>,
    /// Months until the stated monthly amount reaches the target. `None` when no amount is
    /// stated, or when this pace does not arrive within a hundred years.
    pub months_to_target: Option<u32>,
    /// The month `months_to_target` lands on; `None` for the same reasons.
    pub projected_date: Option<NaiveDate>,
    /// Whether the stated monthly amount is at least the required one. `None` when either is
    /// missing: an unanswerable question is not a "no".
    pub on_track: Option<bool>,
    /// Echoed back so a tile can say what it assumed without keeping its own copy.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub monthly_base: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub expected_return: Decimal,
}

/// Measures `goal` against `current_base` at `as_of`.
///
/// The goal's own currency is converted here rather than by the caller, so an amount typed in
/// one currency and a portfolio reported in another cannot be compared by accident.
pub fn goal_progress(
    goal: &Goal,
    current_base: Decimal,
    base: &str,
    as_of: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<GoalProgress> {
    goal.validate()?;
    let target_base = rates.convert(goal.target_amount, &goal.currency, base, as_of)?;
    let monthly_base = goal
        .monthly_amount
        .map(|amount| rates.convert(amount, &goal.currency, base, as_of))
        .transpose()?;

    let months_left = goal.target_date.map(|date| months_between(as_of, date));
    let required_monthly_base = months_left.map(|months| {
        monthly_needed(current_base, target_base, months, goal.expected_return).unwrap_or(Decimal::ZERO)
    });
    let months_to_target = monthly_base
        .and_then(|monthly| months_needed(current_base, target_base, monthly, goal.expected_return));
    let projected_date = months_to_target.and_then(|n| as_of.checked_add_months(chrono::Months::new(n)));

    Ok(GoalProgress {
        goal_id: goal.id.clone(),
        name: goal.name.clone(),
        missing_base: (target_base - current_base).max(Decimal::ZERO),
        progress: if target_base.is_zero() {
            Decimal::ZERO
        } else {
            current_base / target_base
        },
        months_left,
        on_track: match (monthly_base, required_monthly_base) {
            (Some(stated), Some(required)) => Some(stated >= required),
            _ => None,
        },
        required_monthly_base,
        months_to_target,
        projected_date,
        target_base,
        current_base,
        monthly_base,
        expected_return: goal.expected_return,
    })
}

/// Whole months from `from` to `to`, floored at zero: a deadline already passed leaves no time,
/// which is not the same as time running backwards.
fn months_between(from: NaiveDate, to: NaiveDate) -> u32 {
    if to <= from {
        return 0;
    }
    let months = (to.year() - from.year()) * 12 + to.month() as i32 - from.month() as i32;
    // The day of the month decides whether the last one has actually gone by.
    let months = if to.day() < from.day() { months - 1 } else { months };
    months.max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// One pair and nothing else; the same currency is always one.
    struct Rates(Option<(&'static str, &'static str, Decimal)>);
    impl RateLookup for Rates {
        fn rate_as_of(&self, from: &str, to: &str, _date: NaiveDate) -> Result<Option<Decimal>> {
            if from == to {
                return Ok(Some(Decimal::ONE));
            }
            Ok(self
                .0
                .filter(|(a, b, _)| *a == from && *b == to)
                .map(|(_, _, rate)| rate))
        }
    }

    const NO_RATES: Rates = Rates(None);

    fn goal() -> Goal {
        let mut goal = Goal::new("Flat deposit", dec!(20000), "EUR", d(2026, 1, 1));
        goal.target_date = Some(d(2027, 1, 1));
        goal
    }

    /// 8 000 saved of 20 000, twelve months left and nothing growing: 12 000 / 12 = 1 000 a month.
    #[test]
    fn a_dated_goal_says_what_it_takes_each_month() {
        let progress = goal_progress(&goal(), dec!(8000), "EUR", d(2026, 1, 1), &NO_RATES).unwrap();

        assert_eq!(progress.target_base, dec!(20000));
        assert_eq!(progress.missing_base, dec!(12000));
        assert_eq!(progress.progress, dec!(0.4));
        assert_eq!(progress.months_left, Some(12));
        assert_eq!(progress.required_monthly_base.unwrap().round_dp(2), dec!(1000));
        // Nothing was stated, so there is nothing to be on track with.
        assert_eq!(progress.on_track, None);
        assert_eq!(progress.months_to_target, None);
    }

    /// Paying 1 200 a month against a 1 000 requirement is on track, and arrives in ten months.
    #[test]
    fn a_stated_pace_answers_when_it_arrives() {
        let mut goal = goal();
        goal.monthly_amount = Some(dec!(1200));
        let progress = goal_progress(&goal, dec!(8000), "EUR", d(2026, 1, 1), &NO_RATES).unwrap();

        assert_eq!(progress.on_track, Some(true));
        assert_eq!(progress.months_to_target, Some(10));
        assert_eq!(progress.projected_date, Some(d(2026, 11, 1)));
    }

    /// A goal already met needs nothing a month, and is not "behind" for having no pace.
    #[test]
    fn a_met_goal_needs_nothing() {
        let progress = goal_progress(&goal(), dec!(25000), "EUR", d(2026, 1, 1), &NO_RATES).unwrap();

        assert_eq!(progress.missing_base, Decimal::ZERO);
        assert_eq!(progress.progress, dec!(1.25));
        assert_eq!(progress.required_monthly_base, Some(Decimal::ZERO));
    }

    /// A deadline in the past leaves no months, not negative ones.
    #[test]
    fn a_passed_deadline_leaves_no_time() {
        let progress = goal_progress(&goal(), dec!(8000), "EUR", d(2027, 6, 1), &NO_RATES).unwrap();
        assert_eq!(progress.months_left, Some(0));
        // Nothing monthly can close it now, so the requirement is the whole gap.
        assert_eq!(progress.required_monthly_base, Some(dec!(12000)));
    }

    /// A goal typed in another currency is converted at the reading date, once.
    #[test]
    fn a_foreign_goal_is_converted_before_it_is_compared() {
        let mut goal = goal();
        goal.currency = crate::money::normalize_currency("USD");
        let rates = Rates(Some(("USD", "EUR", dec!(0.9))));

        let progress = goal_progress(&goal, dec!(9000), "EUR", d(2026, 1, 1), &rates).unwrap();
        assert_eq!(progress.target_base, dec!(18000.0));
        assert_eq!(progress.progress, dec!(0.5));
    }
}
