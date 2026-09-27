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
mod tests;
