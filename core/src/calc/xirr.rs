use crate::calc::CashFlow;
use crate::error::{Error, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// Newton iteration cap; protects pathological cash-flow sets from looping.
pub const XIRR_MAX_ITERATIONS: usize = 100;

/// NPV tolerance used to accept a root.
const NPV_EPSILON: f64 = 1e-9;
/// Days per year for Excel-compatible XIRR discounting.
const DAYS_PER_YEAR: f64 = 365.0;

/// Solves `NPV(r) = Σ CF_i / (1 + r)^(d_i / 365) = 0` for irregular dates.
/// Uses `f64` only for the rate, with Newton first and bisection fallback.
pub fn xirr(flows: &[CashFlow]) -> Result<Decimal> {
    if flows.len() < 2 {
        return Err(Error::Math("XIRR needs at least two cash flows".into()));
    }

    let start = flows.iter().map(|f| f.date).min().expect("non-empty");
    // Precompute year offsets and amounts so the solver does not revisit dates.
    let series: Vec<(f64, f64)> = flows
        .iter()
        .map(|f| {
            let years = (f.date - start).num_days() as f64 / DAYS_PER_YEAR;
            let amount = f.amount_base.to_f64().unwrap_or(0.0);
            (years, amount)
        })
        .collect();

    // A root requires both positive and negative flows.
    let has_positive = series.iter().any(|(_, a)| *a > 0.0);
    let has_negative = series.iter().any(|(_, a)| *a < 0.0);
    if !(has_positive && has_negative) {
        return Err(Error::Math(
            "XIRR requires both positive and negative cash flows".into(),
        ));
    }

    if let Some(rate) = newton(&series) {
        return to_decimal(rate);
    }
    match bisect(&series) {
        Some(rate) => to_decimal(rate),
        None => Err(Error::Math("XIRR did not converge".into())),
    }
}

/// NPV at `rate`.
fn npv(series: &[(f64, f64)], rate: f64) -> f64 {
    series
        .iter()
        .map(|(years, amount)| amount / (1.0 + rate).powf(*years))
        .sum()
}

/// NPV derivative used by Newton's method.
fn npv_derivative(series: &[(f64, f64)], rate: f64) -> f64 {
    series
        .iter()
        .map(|(years, amount)| -years * amount / (1.0 + rate).powf(years + 1.0))
        .sum()
}

fn newton(series: &[(f64, f64)]) -> Option<f64> {
    let mut rate = 0.1;
    for _ in 0..XIRR_MAX_ITERATIONS {
        let value = npv(series, rate);
        if value.abs() < NPV_EPSILON {
            return Some(rate);
        }
        let slope = npv_derivative(series, rate);
        if slope.abs() < f64::EPSILON {
            return None;
        }
        let next = rate - value / slope;
        // At or below -100%, fractional powers are undefined.
        if !next.is_finite() || next <= -0.999_999 {
            return None;
        }
        if (next - rate).abs() < 1e-12 {
            return Some(next);
        }
        rate = next;
    }
    None
}

/// Bisection over -99.99% to +100,000%; high returns can be thousands of percent.
fn bisect(series: &[(f64, f64)]) -> Option<f64> {
    let (mut low, mut high) = (-0.9999_f64, 1000.0_f64);
    let (mut f_low, f_high) = (npv(series, low), npv(series, high));
    if f_low * f_high > 0.0 {
        return None;
    }
    for _ in 0..200 {
        let mid = (low + high) / 2.0;
        let f_mid = npv(series, mid);
        if f_mid.abs() < NPV_EPSILON || (high - low) < 1e-12 {
            return Some(mid);
        }
        if f_low * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
            f_low = f_mid;
        }
    }
    Some((low + high) / 2.0)
}

/// Convert back to `Decimal`; ten places avoid exposing meaningless `f64` noise.
fn to_decimal(rate: f64) -> Result<Decimal> {
    Decimal::from_f64(rate)
        .map(|d| d.round_dp(10))
        .ok_or_else(|| Error::Math(format!("rate {rate} is not representable")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn flow(y: i32, m: u32, d: u32, amount: Decimal) -> CashFlow {
        CashFlow {
            date: NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            amount_base: amount,
        }
    }

    /// Invest 1000 and receive 1100 after 365 days: 10% annualized.
    #[test]
    fn one_year_ten_percent() {
        let flows = vec![flow(2023, 1, 1, dec!(-1000)), flow(2024, 1, 1, dec!(1100))];
        let r = xirr(&flows).unwrap();
        assert_eq!(r.round_dp(6), dec!(0.1));
    }

    /// A 10% gain in 182 days annualizes to `(1.1)^(365/182) - 1`.
    #[test]
    fn half_year_annualizes_upward() {
        let flows = vec![flow(2023, 1, 1, dec!(-1000)), flow(2023, 7, 2, dec!(1100))];
        let r = xirr(&flows).unwrap();
        let expected = 1.1_f64.powf(365.0 / 182.0) - 1.0;
        let diff = (r.to_f64().unwrap() - expected).abs();
        assert!(diff < 1e-6, "got {r}, expected ≈ {expected}");
    }

    /// A 1000 -> 900 loss over a year yields -10%.
    #[test]
    fn loss_gives_negative_rate() {
        let flows = vec![flow(2023, 1, 1, dec!(-1000)), flow(2024, 1, 1, dec!(900))];
        assert_eq!(xirr(&flows).unwrap().round_dp(6), dec!(-0.1));
    }

    /// Multiple contributions: the solved rate must zero the NPV.
    #[test]
    fn multiple_contributions_zero_the_npv() {
        let flows = vec![
            flow(2022, 1, 1, dec!(-1000)),
            flow(2022, 7, 1, dec!(-500)),
            flow(2023, 3, 15, dec!(-250)),
            flow(2024, 1, 1, dec!(2000)),
        ];
        let r = xirr(&flows).unwrap().to_f64().unwrap();
        let start = flows.iter().map(|f| f.date).min().unwrap();
        let series: Vec<(f64, f64)> = flows
            .iter()
            .map(|f| {
                (
                    (f.date - start).num_days() as f64 / 365.0,
                    f.amount_base.to_f64().unwrap(),
                )
            })
            .collect();
        assert!(npv(&series, r).abs() < 1e-6);
    }

    #[test]
    fn rejects_flows_without_sign_change() {
        let flows = vec![flow(2023, 1, 1, dec!(100)), flow(2024, 1, 1, dec!(100))];
        assert!(matches!(xirr(&flows), Err(Error::Math(_))));
    }
}
