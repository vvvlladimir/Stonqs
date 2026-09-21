//! All-time high and the distance below it.
//!
//! This one is measured on **value**, not on chained returns the way [`super::drawdowns`] is.
//! The two answer different questions: a drawdown asks how the instruments did, an all-time
//! high asks what the statement said on its best day — and a deposit does raise that.

use super::ValueSeries;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The highest point of a series and where the last point stands against it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peak {
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub value: Decimal,
    /// The last point of the series — what the peak is being compared with.
    #[serde(with = "rust_decimal::serde::str")]
    pub current: Decimal,
    /// `current / peak - 1`: zero at the high, negative below it. `None` when the peak is
    /// zero or negative, where a percentage of it would mean nothing.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub distance: Option<Decimal>,
    pub days_since: i64,
}

/// Highest point of a dated series; the first of equal highs wins, so the date is the one
/// the level was first reached.
pub fn peak_of<I>(points: I) -> Option<Peak>
where
    I: IntoIterator<Item = (NaiveDate, Decimal)>,
{
    let mut best: Option<(NaiveDate, Decimal)> = None;
    let mut last: Option<(NaiveDate, Decimal)> = None;
    for (date, value) in points {
        if best.is_none_or(|(_, high)| value > high) {
            best = Some((date, value));
        }
        last = Some((date, value));
    }
    let (date, value) = best?;
    let (last_date, current) = last?;
    Some(Peak {
        date,
        value,
        current,
        distance: (value > Decimal::ZERO).then(|| current / value - Decimal::ONE),
        days_since: (last_date - date).num_days(),
    })
}

/// Peak of the portfolio value over the series' own range.
pub fn all_time_high(series: &ValueSeries) -> Option<Peak> {
    peak_of(
        series
            .dates
            .iter()
            .copied()
            .zip(series.total_value_base.iter().copied()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 6, day).unwrap()
    }

    /// Peak 1200 on the 3rd, last value 1080: 1080/1200 - 1 = -0.10, seven days later.
    #[test]
    fn the_distance_is_measured_from_the_highest_point() {
        let peak = peak_of([
            (day(1), dec!(1000)),
            (day(3), dec!(1200)),
            (day(5), dec!(900)),
            (day(10), dec!(1080)),
        ])
        .unwrap();

        assert_eq!(peak.date, day(3));
        assert_eq!(peak.value, dec!(1200));
        assert_eq!(peak.distance, Some(dec!(-0.10)));
        assert_eq!(peak.days_since, 7);
    }

    /// A series still at its high stands zero below it, not "no drawdown".
    #[test]
    fn a_new_high_is_zero_below_itself() {
        let peak = peak_of([(day(1), dec!(1000)), (day(2), dec!(1500))]).unwrap();

        assert_eq!(peak.date, day(2));
        assert_eq!(peak.distance, Some(Decimal::ZERO));
        assert_eq!(peak.days_since, 0);
    }

    /// An empty portfolio has no high to be below.
    #[test]
    fn an_empty_series_has_no_peak() {
        assert_eq!(peak_of(Vec::<(NaiveDate, Decimal)>::new()), None);
    }
}
