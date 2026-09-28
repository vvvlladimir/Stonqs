use crate::error::{Error, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// Sub-period boundary: end-of-day value plus the flow entering the next period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TwrPoint {
    pub date: NaiveDate,
    /// End-of-day portfolio value in the base currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub end_value: Decimal,
    /// Flow after the previous point and at or before this period's start; `+` is a deposit.
    #[serde(with = "rust_decimal::serde::str")]
    pub external_flow: Decimal,
}

/// `r_i = V_i / (V_{i-1} + F_i) - 1`, chained; the first point's flow is ignored. A sub-period
/// with no positive capital to start from is skipped rather than chained, which is also what
/// keeps the result above −100% (ADR-0090).
pub fn time_weighted_return(points: &[TwrPoint]) -> Result<Decimal> {
    if points.len() < 2 {
        return Err(Error::Math("TWR needs at least two points".into()));
    }

    let mut cumulative = Decimal::ONE;
    let mut chained = 0usize;
    for window in points.windows(2) {
        let (prev, cur) = (&window[0], &window[1]);
        let start_capital = prev.end_value + cur.external_flow;

        // A flow that empties the holding is measured against the value before it; otherwise a
        // sale would read as −100%.
        if cur.end_value.is_zero() && cur.external_flow.is_sign_negative() {
            if prev.end_value <= Decimal::ZERO {
                continue;
            }
            cumulative = chain(cumulative, -cur.external_flow, prev.end_value, cur.date)?;
            chained += 1;
            continue;
        }

        // Only capital the portfolio actually had can earn a return, and a portfolio that owes
        // more than it holds is a ledger missing its deposits, not a loss of more than
        // everything. Either way the sub-period is skipped and the chain resumes at the next one
        // with both ends meaning something (ADR-0090).
        if start_capital <= Decimal::ZERO || cur.end_value.is_sign_negative() {
            continue;
        }

        cumulative = chain(cumulative, cur.end_value, start_capital, cur.date)?;
        chained += 1;
    }

    // Every sub-period was skipped while the window did hold something: a return of 0 would read
    // as "it went nowhere", and what happened is that no capital this ledger accounts for was
    // ever at work. A window in which the portfolio was simply empty keeps its flat 0.
    let held_something = points
        .iter()
        .any(|point| !point.end_value.is_zero() || !point.external_flow.is_zero());
    if chained == 0 && held_something {
        return Err(Error::Math(
            "no sub-period of the window starts with capital to earn on".into(),
        ));
    }
    Ok(cumulative - Decimal::ONE)
}

/// One sub-period's factor, chained. A capital of dust against a value of billions is a growth
/// factor no number holds: it is a return that cannot be stated, which is an error and never a
/// panic in the middle of a screen.
fn chain(cumulative: Decimal, value: Decimal, capital: Decimal, date: NaiveDate) -> Result<Decimal> {
    value
        .checked_div(capital)
        .and_then(|factor| cumulative.checked_mul(factor))
        .ok_or_else(|| {
            Error::Math(format!(
                "sub-period ending {date} grows {capital} into {value}, which does not fit"
            ))
        })
}

/// Calendar days, not trading days: a period is a stretch of the calendar.
const DAYS_PER_YEAR: f64 = 365.0;

/// `(1 + twr)^(365 / days) - 1`, `f64` for `powf` (a rate, not money). `None` under a day or at a total loss.
pub fn annualize(twr: Decimal, from: NaiveDate, to: NaiveDate) -> Option<Decimal> {
    let days = (to - from).num_days();
    if days <= 0 {
        return None;
    }
    let growth = (Decimal::ONE + twr).to_f64()?;
    if growth <= 0.0 {
        return None;
    }
    Decimal::from_f64_retain(growth.powf(DAYS_PER_YEAR / days as f64) - 1.0).map(|r| r.round_dp(10))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn p(date: NaiveDate, end: Decimal, flow: Decimal) -> TwrPoint {
        TwrPoint {
            date,
            end_value: end,
            external_flow: flow,
        }
    }

    /// Without flows, TWR is ordinary return: 1000 -> 1100 is +10%.
    #[test]
    fn simple_growth_without_flows() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 12, 31), dec!(1100), Decimal::ZERO),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(0.1));
    }

    /// A deposit is not profit: 1000 + 1000 -> 2000 gives TWR 0%, not +100%.
    #[test]
    fn deposit_is_not_profit() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 6, 30), dec!(2000), dec!(1000)),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), Decimal::ZERO);
    }

    /// Geometric chaining: 1000 -> 1100, then 900 added, then 2000 -> 2200;
    /// `1.1 * 1.1 - 1 = 0.21`.
    #[test]
    fn chains_sub_periods_geometrically() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 6, 30), dec!(1100), Decimal::ZERO),
            p(d(2024, 12, 31), dec!(2200), dec!(900)),
        ];
        let twr = time_weighted_return(&points).unwrap();
        assert_eq!(twr.round_dp(10), dec!(0.21));
    }

    /// A withdrawal reduces starting capital: 1000 - 500 -> 550 is +10%.
    #[test]
    fn withdrawal_reduces_starting_capital() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 12, 31), dec!(550), dec!(-500)),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(0.1));
    }

    /// Losses chain geometrically: 1000 -> 500 -> 750 gives `0.5 * 1.5 - 1 = -0.25`.
    #[test]
    fn losses_chain_too() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 6, 30), dec!(500), Decimal::ZERO),
            p(d(2024, 12, 31), dec!(750), Decimal::ZERO),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(-0.25));
    }

    /// Selling a holding out is not a total loss: worth 1000 at the last close, sold for
    /// 1050, empty afterwards; the sub-period is 1050 / 1000 = +5%, and the empty tail that
    /// follows changes nothing.
    #[test]
    fn liquidation_measures_proceeds_against_the_last_value() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 6, 30), Decimal::ZERO, dec!(-1050)),
            p(d(2024, 12, 31), Decimal::ZERO, Decimal::ZERO),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(0.05));
    }

    /// A holding that really did go to zero keeps its loss: worth 1000, a 20 dividend on the
    /// way, nothing left; 20 / 1000 - 1 = -98%.
    #[test]
    fn a_position_that_goes_worthless_still_loses() {
        let points = vec![
            p(d(2024, 1, 1), dec!(1000), Decimal::ZERO),
            p(d(2024, 6, 30), Decimal::ZERO, dec!(-20)),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(-0.98));
    }

    /// An empty leading period is skipped instead of breaking the calculation.
    #[test]
    fn empty_leading_period_is_skipped() {
        let points = vec![
            p(d(2024, 1, 1), Decimal::ZERO, Decimal::ZERO),
            p(d(2024, 1, 2), Decimal::ZERO, Decimal::ZERO),
            p(d(2024, 12, 31), dec!(1100), dec!(1000)),
        ];
        assert_eq!(time_weighted_return(&points).unwrap(), dec!(0.1));
    }

    /// Value appearing with no flow to explain it is a hole in the ledger — a broker export of
    /// trades alone, with the deposits that paid for them in another file. Nothing was chained,
    /// so the return is not stated: a 0 % would read as "it went nowhere" (ADR-0090).
    #[test]
    fn value_out_of_nowhere_leaves_the_return_unstated() {
        let points = vec![
            p(d(2024, 1, 1), Decimal::ZERO, Decimal::ZERO),
            p(d(2024, 12, 31), dec!(500), Decimal::ZERO),
        ];
        assert!(matches!(time_weighted_return(&points), Err(Error::Math(_))));
    }

    /// Purchases with no deposit behind them leave the cash account below zero, so a sub-period
    /// can start — or end — with the portfolio owing more than it holds. Chained, that turns the
    /// factor negative and the whole return drops below −100%, which no long-only portfolio can
    /// do: those sub-periods are skipped, and 400 growing to 500 afterwards is the +25% that
    /// remains.
    #[test]
    fn negative_capital_never_drags_the_return_below_minus_one() {
        let points = vec![
            p(d(2024, 1, 1), dec!(100), Decimal::ZERO),
            p(d(2024, 3, 31), dec!(-100), Decimal::ZERO),
            p(d(2024, 6, 30), dec!(400), Decimal::ZERO),
            p(d(2024, 12, 31), dec!(500), Decimal::ZERO),
        ];
        let twr = time_weighted_return(&points).unwrap();
        assert_eq!(twr, dec!(0.25));
        assert!(twr > dec!(-1));
    }

    /// Two years at +21% total: `1.21^(365/730) - 1`. The period is 730 days, so the
    /// exponent is exactly 0.5 and the yearly rate is `sqrt(1.21) - 1 = 0.10`.
    #[test]
    fn annualizes_a_two_year_return_to_its_square_root() {
        let rate = annualize(dec!(0.21), d(2023, 1, 1), d(2024, 12, 31)).unwrap();
        assert_eq!(rate.round_dp(6), dec!(0.1));
    }

    /// Under a year the rate is scaled up: +5% over 73 days is `1.05^5 - 1 = 0.2762815625`.
    #[test]
    fn a_short_period_is_scaled_up_not_left_alone() {
        let rate = annualize(dec!(0.05), d(2024, 1, 1), d(2024, 3, 14)).unwrap();
        assert_eq!(rate.round_dp(8), dec!(0.27628156));
    }

    /// A period of one year returns the period figure unchanged.
    #[test]
    fn one_year_is_itself() {
        let rate = annualize(dec!(0.17), d(2024, 1, 1), d(2024, 12, 31)).unwrap();
        assert_eq!(rate.round_dp(6), dec!(0.17));
    }

    #[test]
    fn a_total_loss_and_an_empty_period_have_no_yearly_rate() {
        assert_eq!(annualize(dec!(-1), d(2023, 1, 1), d(2024, 1, 1)), None);
        assert_eq!(annualize(dec!(0.1), d(2024, 1, 1), d(2024, 1, 1)), None);
    }
}
