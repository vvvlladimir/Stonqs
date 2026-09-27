use super::*;
use crate::money::normalize_currency;
use rust_decimal_macros::dec;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// Value series without external flows.
fn series(values: &[(NaiveDate, Decimal)]) -> ValueSeries {
    ValueSeries {
        base_currency: normalize_currency("EUR"),
        dates: values.iter().map(|(d, _)| *d).collect(),
        total_value_base: values.iter().map(|(_, v)| *v).collect(),
        external_flow_base: values.iter().map(|_| Decimal::ZERO).collect(),
    }
}

/// Monthly chaining: Jan 31 = 100, Feb 29 = 110, Mar 31 = 99;
/// `1.0 * 1.1 * 0.9 - 1 = -0.01`, equal to `99 / 100 - 1`.
#[test]
fn monthly_returns_chain_into_the_whole_period() {
    let s = series(&[
        (d(2024, 1, 31), dec!(100)),
        (d(2024, 2, 29), dec!(110)),
        (d(2024, 3, 31), dec!(99)),
    ]);
    let months = returns_by_period(&s, Period::Month);
    assert_eq!(months.len(), 3);
    assert_eq!(months[0].twr, Decimal::ZERO);
    assert_eq!(months[1].twr, dec!(0.1));
    assert_eq!(months[2].from, d(2024, 3, 1));
    assert_eq!(months[2].twr, dec!(-0.1));

    let chained = months
        .iter()
        .fold(Decimal::ONE, |acc, m| acc * (Decimal::ONE + m.twr))
        - Decimal::ONE;
    assert_eq!(chained, s.twr().unwrap());
}

/// A mid-month deposit is not monthly return: 1000 + 1000 -> 2000 gives 0%.
#[test]
fn a_deposit_inside_the_month_is_not_a_return() {
    let s = ValueSeries {
        base_currency: normalize_currency("EUR"),
        dates: vec![d(2024, 3, 31), d(2024, 4, 15), d(2024, 4, 30)],
        total_value_base: vec![dec!(1000), dec!(2000), dec!(2000)],
        external_flow_base: vec![Decimal::ZERO, dec!(1000), Decimal::ZERO],
    };
    let april = returns_by_period(&s, Period::Month)
        .into_iter()
        .find(|p| p.from == d(2024, 4, 1))
        .unwrap();
    assert_eq!(april.twr, Decimal::ZERO);
}

/// A year is split into 12 calendar months.
#[test]
fn splits_a_year_into_calendar_months() {
    let months = Period::Month.split(DateRange::new(d(2024, 1, 1), d(2024, 12, 31)));
    assert_eq!(months.len(), 12);
    assert_eq!(months[0], DateRange::new(d(2024, 1, 1), d(2024, 1, 31)));
    // 2024 is leap year: February ends on the 29th.
    assert_eq!(months[1], DateRange::new(d(2024, 2, 1), d(2024, 2, 29)));
    assert_eq!(months[11], DateRange::new(d(2024, 12, 1), d(2024, 12, 31)));
}

/// Partial edges are clipped without shifting the period grid.
#[test]
fn truncates_partial_edges() {
    let months = Period::Month.split(DateRange::new(d(2024, 3, 15), d(2024, 5, 10)));
    assert_eq!(months.len(), 3);
    assert_eq!(months[0], DateRange::new(d(2024, 3, 15), d(2024, 3, 31)));
    assert_eq!(months[1], DateRange::new(d(2024, 4, 1), d(2024, 4, 30)));
    assert_eq!(months[2], DateRange::new(d(2024, 5, 1), d(2024, 5, 10)));
}

/// Quarters end on March 31, June 30, September 30, and December 31.
#[test]
fn quarters_end_on_calendar_boundaries() {
    let q = Period::Quarter.split(DateRange::new(d(2024, 2, 1), d(2024, 12, 31)));
    assert_eq!(q.len(), 4);
    assert_eq!(q[0], DateRange::new(d(2024, 2, 1), d(2024, 3, 31)));
    assert_eq!(q[3], DateRange::new(d(2024, 10, 1), d(2024, 12, 31)));
}

/// Weeks end on Sunday; June 3, 2024 is a Monday.
#[test]
fn weeks_end_on_sunday() {
    let w = Period::Week.split(DateRange::new(d(2024, 6, 5), d(2024, 6, 20)));
    assert_eq!(w[0], DateRange::new(d(2024, 6, 5), d(2024, 6, 9)));
    assert_eq!(w[1], DateRange::new(d(2024, 6, 10), d(2024, 6, 16)));
}

#[test]
fn ytd_starts_on_first_january() {
    let r = PeriodPreset::Ytd.range(d(2024, 6, 5), None).unwrap();
    assert_eq!(r, DateRange::new(d(2024, 1, 1), d(2024, 6, 5)));
}

/// 5 Jun 2024 minus one month is 5 May 2024; minus three months is 5 Mar 2024.
#[test]
fn month_presets_keep_the_day_of_month() {
    let r = PeriodPreset::OneMonth.range(d(2024, 6, 5), None).unwrap();
    assert_eq!(r, DateRange::new(d(2024, 5, 5), d(2024, 6, 5)));

    let r = PeriodPreset::ThreeMonths.range(d(2024, 6, 5), None).unwrap();
    assert_eq!(r, DateRange::new(d(2024, 3, 5), d(2024, 6, 5)));
}

/// 31 Mar 2024 minus one month has no 31 Feb: February 2024 ends on the 29th.
#[test]
fn a_month_back_clamps_to_the_last_day() {
    let r = PeriodPreset::OneMonth.range(d(2024, 3, 31), None).unwrap();
    assert_eq!(r, DateRange::new(d(2024, 2, 29), d(2024, 3, 31)));
}

/// 15 Jan 2024 minus three months crosses the year end: 15 Oct 2023.
#[test]
fn months_back_cross_the_year_boundary() {
    let r = PeriodPreset::ThreeMonths.range(d(2024, 1, 15), None).unwrap();
    assert_eq!(r, DateRange::new(d(2023, 10, 15), d(2024, 1, 15)));
}

/// Presets never start before the first transaction.
#[test]
fn preset_is_clamped_to_inception() {
    let r = PeriodPreset::FiveYears
        .range(d(2024, 6, 5), Some(d(2023, 3, 1)))
        .unwrap();
    assert_eq!(r, DateRange::new(d(2023, 3, 1), d(2024, 6, 5)));
}

#[test]
fn since_inception_without_transactions_is_an_error() {
    assert!(matches!(
        PeriodPreset::SinceInception.range(d(2024, 6, 5), None),
        Err(Error::Invalid(_))
    ));
}

/// A window back from a date is the same arithmetic the presets use: 6 months back from
/// 2024-08-31 is 2024-02-29 (the shorter month clamps the day), two quarters back is the
/// same date, and a week back is exactly seven days.
#[test]
fn a_unit_names_a_window_back_as_well_as_a_chunk() {
    assert_eq!(Period::Month.before(d(2024, 8, 31), 6).unwrap(), d(2024, 2, 29));
    assert_eq!(Period::Quarter.before(d(2024, 8, 31), 2).unwrap(), d(2024, 2, 29));
    assert_eq!(Period::Week.before(d(2024, 8, 31), 1).unwrap(), d(2024, 8, 24));
    assert_eq!(Period::Day.before(d(2024, 8, 31), 30).unwrap(), d(2024, 8, 1));
    assert_eq!(Period::Year.before(d(2024, 2, 29), 1).unwrap(), d(2023, 2, 28));
}

/// A user's relative window gets the same inception clamp a preset does: 10 years back
/// from a portfolio that opened in 2022 is 2022, not a decade of emptiness.
#[test]
fn a_relative_window_is_clamped_to_the_first_transaction() {
    let spec = PeriodSpec::Relative {
        unit: Period::Year,
        count: 10,
    };
    let range = spec.range(d(2026, 9, 14), Some(d(2022, 3, 1))).unwrap();
    assert_eq!(range.from, d(2022, 3, 1));
    assert_eq!(range.to, d(2026, 9, 14));
}

/// A fixed window stays where it was put, but never reaches past the reporting date:
/// "2026" viewed on 2026-09-14 ends that day, because no quote exists beyond it.
#[test]
fn a_fixed_window_stops_at_the_reporting_date() {
    let spec = PeriodSpec::Fixed {
        from: d(2026, 1, 1),
        to: Some(d(2026, 12, 31)),
    };
    let range = spec.range(d(2026, 9, 14), Some(d(2022, 3, 1))).unwrap();
    assert_eq!(range.from, d(2026, 1, 1));
    assert_eq!(range.to, d(2026, 9, 14));
}

/// A window entirely before the portfolio existed has nothing to show, and saying so is
/// better than clamping it into a backwards range.
#[test]
fn a_fixed_window_before_the_first_transaction_is_rejected() {
    let spec = PeriodSpec::Fixed {
        from: d(2019, 1, 1),
        to: Some(d(2019, 12, 31)),
    };
    assert!(spec.range(d(2026, 9, 14), Some(d(2022, 3, 1))).is_err());
    // Without any history there is nothing to clamp against, so the window stands.
    assert!(spec.range(d(2026, 9, 14), None).is_ok());
}

/// A start with no end runs to the reporting date, so the window grows by itself: written
/// on 2024-05-12, it covers 2024-05-12..2026-09-14 when viewed on 2026-09-14.
#[test]
fn an_open_ended_window_runs_to_the_reporting_date() {
    let spec = PeriodSpec::Fixed {
        from: d(2024, 5, 12),
        to: None,
    };
    let range = spec.range(d(2026, 9, 14), Some(d(2022, 3, 1))).unwrap();
    assert_eq!(range, DateRange::new(d(2024, 5, 12), d(2026, 9, 14)));
}

/// An open end does not save a window that starts after the reporting date.
#[test]
fn an_open_ended_window_cannot_start_in_the_future() {
    let spec = PeriodSpec::Fixed {
        from: d(2027, 1, 1),
        to: None,
    };
    assert!(spec.range(d(2026, 9, 14), None).is_err());
}

#[test]
fn a_relative_window_of_zero_units_is_rejected() {
    let spec = PeriodSpec::Relative {
        unit: Period::Month,
        count: 0,
    };
    assert!(spec.range(d(2026, 9, 14), None).is_err());
}
