use super::ValueSeries;
use crate::error::{Error, Result};
use crate::market::DateRange;
use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Calendar granularity to split a period by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Period {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl Period {
    /// Splits a range into calendar-aligned week, month, quarter, or year chunks; edge
    /// chunks are truncated to the requested range.
    pub fn split(self, range: DateRange) -> Vec<DateRange> {
        let mut out = Vec::new();
        if range.to < range.from {
            return out;
        }
        let mut start = range.from;
        loop {
            let end = self.period_end(start).min(range.to);
            out.push(DateRange::new(start, end));
            if end >= range.to {
                break;
            }
            start = end + Duration::days(1);
        }
        out
    }

    /// The same calendar point `count` units earlier. A quarter is three months and a week is
    /// seven days, so the unit that splits a range also names a window back from one.
    pub fn before(self, date: NaiveDate, count: u32) -> Result<NaiveDate> {
        match self {
            Period::Day => date
                .checked_sub_signed(Duration::days(count as i64))
                .ok_or_else(|| Error::Math(format!("cannot go {count} days back from {date}"))),
            Period::Week => date
                .checked_sub_signed(Duration::days(count as i64 * 7))
                .ok_or_else(|| Error::Math(format!("cannot go {count} weeks back from {date}"))),
            Period::Month => months_before(date, count),
            Period::Quarter => months_before(date, count * 3),
            Period::Year => years_before(date, count as i32),
        }
    }

    /// Last day of the period that `date` falls in.
    fn period_end(self, date: NaiveDate) -> NaiveDate {
        match self {
            Period::Day => date,
            // num_days_from_monday() is 0 on Monday, so Sunday is exactly `6 - n` days away.
            Period::Week => date + Duration::days(6 - date.weekday().num_days_from_monday() as i64),
            Period::Month => last_day_of_month(date.year(), date.month()),
            Period::Quarter => {
                let last_month = (date.month() - 1) / 3 * 3 + 3;
                last_day_of_month(date.year(), last_month)
            }
            Period::Year => last_day_of_month(date.year(), 12),
        }
    }
}

/// UI period presets: 1/3 months, YTD and 1/3/5 years, with since-inception covering all history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodPreset {
    OneMonth,
    ThreeMonths,
    Ytd,
    OneYear,
    ThreeYears,
    FiveYears,
    /// From the first transaction to `as_of`.
    SinceInception,
}

impl PeriodPreset {
    /// Resolves a preset range as of `as_of`; `inception` is used for since-inception.
    pub fn range(self, as_of: NaiveDate, inception: Option<NaiveDate>) -> Result<DateRange> {
        let from = match self {
            PeriodPreset::OneMonth => Period::Month.before(as_of, 1)?,
            PeriodPreset::ThreeMonths => Period::Month.before(as_of, 3)?,
            PeriodPreset::Ytd => NaiveDate::from_ymd_opt(as_of.year(), 1, 1)
                .ok_or_else(|| Error::Math(format!("cannot build 1 Jan {}", as_of.year())))?,
            PeriodPreset::OneYear => Period::Year.before(as_of, 1)?,
            PeriodPreset::ThreeYears => Period::Year.before(as_of, 3)?,
            PeriodPreset::FiveYears => Period::Year.before(as_of, 5)?,
            PeriodPreset::SinceInception => inception.ok_or_else(|| {
                Error::Invalid("SinceInception needs the date of the first transaction".into())
            })?,
        };
        Ok(clamp(from, as_of, as_of, inception))
    }
}

/// A period the user defined: a window back from today, or two dates written down.
///
/// The seven [`PeriodPreset`]s stay a closed, core-owned list — see ADR-0018 — because their
/// wording, their cache key and their stored id all have one owner. This is the open half of
/// the same axis: the *shape* is still core's (the arithmetic and the inception clamp live
/// here), only the numbers come from the user. See ADR-0026.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodSpec {
    /// `count` units back from the reporting date; it moves with the calendar.
    Relative { unit: Period, count: u32 },
    /// A start written down, and an end that is either written down too or left open. An open
    /// end means "up to the reporting date" — half a fixed window and half a relative one.
    Fixed {
        from: NaiveDate,
        #[serde(default)]
        to: Option<NaiveDate>,
    },
}

impl PeriodSpec {
    /// Resolves the spec against a reporting date, under the same clamp a preset gets.
    ///
    /// `Err` when nothing is left to show — a fixed window that ends before the portfolio
    /// existed. The caller drops such a period from the strip rather than offering a range
    /// no query can answer.
    pub fn range(self, as_of: NaiveDate, inception: Option<NaiveDate>) -> Result<DateRange> {
        match self {
            PeriodSpec::Relative { unit, count } => {
                if count == 0 {
                    return Err(Error::Invalid("a period of zero units is empty".into()));
                }
                Ok(clamp(unit.before(as_of, count)?, as_of, as_of, inception))
            }
            PeriodSpec::Fixed { from, to } => {
                // `clamp` pulls a start back to the report date, which is right for a preset but
                // would silently turn a window opening next month into "today".
                if from > as_of {
                    return Err(Error::Invalid(format!(
                        "the window starts {from}, after the reporting date {as_of}"
                    )));
                }
                // An open end runs to the report date; a written one still stops there, because
                // no quote exists beyond it.
                let end = to.unwrap_or(as_of).min(as_of);
                let range = clamp(from, end, as_of, inception);
                if range.to < range.from {
                    return Err(Error::Invalid(format!(
                        "the window starting {from} lies outside the portfolio's history"
                    )));
                }
                Ok(range)
            }
        }
    }
}

/// The portfolio may be younger than the request: "5 years" on a one-year-old portfolio
/// means its one year, not four years of emptiness before it.
fn clamp(from: NaiveDate, to: NaiveDate, as_of: NaiveDate, inception: Option<NaiveDate>) -> DateRange {
    let from = match inception {
        Some(start) if start > from => start,
        _ => from,
    };
    DateRange::new(from.min(as_of), to)
}

/// Return of one calendar chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodReturn {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// TWR of the chunk, as a fraction: `0.03` = +3%.
    #[serde(with = "rust_decimal::serde::str")]
    pub twr: Decimal,
}

/// Returns flow-adjusted performance for calendar periods.
/// Chained daily returns preserve TWR composability and real calendar boundaries.
pub fn returns_by_period(series: &ValueSeries, period: Period) -> Vec<PeriodReturn> {
    let (Some(from), Some(to)) = (series.first_date(), series.last_date()) else {
        return Vec::new();
    };
    let returns = series.daily_returns();

    // Both inputs are sorted, so merge them in one pass.
    let mut next = 0;
    period
        .split(DateRange::new(from, to))
        .into_iter()
        .map(|range| {
            let mut growth = Decimal::ONE;
            while next < returns.len() && returns[next].0 <= range.to {
                growth *= Decimal::ONE + returns[next].1;
                next += 1;
            }
            PeriodReturn {
                from: range.from,
                to: range.to,
                twr: growth - Decimal::ONE,
            }
        })
        .collect()
}

/// Last day of a month, derived from the first day of the next month.
fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .expect("month is 1..=12")
        .pred_opt()
        .expect("1 Jan 1 is not representable as a start date here")
}

/// Same calendar date `n` months earlier; a day the shorter month lacks clamps to its last day.
fn months_before(date: NaiveDate, n: u32) -> Result<NaiveDate> {
    // Count months since year 0 so the subtraction crosses January without a special case.
    let months = date.year() * 12 + date.month() as i32 - 1 - n as i32;
    let (year, month) = (months.div_euclid(12), months.rem_euclid(12) as u32 + 1);
    let day = date.day().min(last_day_of_month(year, month).day());
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| Error::Math(format!("cannot go {n} months back from {date}")))
}

/// Same calendar date `n` years earlier; Feb 29 clamps to Feb 28.
fn years_before(date: NaiveDate, n: i32) -> Result<NaiveDate> {
    let year = date.year() - n;
    NaiveDate::from_ymd_opt(year, date.month(), date.day())
        .or_else(|| NaiveDate::from_ymd_opt(year, date.month(), date.day() - 1))
        .ok_or_else(|| Error::Math(format!("cannot go {n} years back from {date}")))
}

#[cfg(test)]
mod tests {
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
}
