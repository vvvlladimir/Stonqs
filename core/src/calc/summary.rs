use super::{ValueSeries, dietz_capital};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The money view of one reporting period, next to the return view a TWR gives.
///
/// A rise in value is not a result: half of it may be a deposit. `absolute_change_base` is what
/// the account statement shows, `delta_base` is what was earned, and `average_capital_base` is
/// the capital that earned it — the denominator every cost rate is measured against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodSummary {
    #[serde(with = "rust_decimal::serde::str")]
    pub start_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub end_value_base: Decimal,
    /// Net external flow over the period; `+` is money brought in.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_flow_base: Decimal,
    /// End minus start: the change a statement reports, deposits included.
    #[serde(with = "rust_decimal::serde::str")]
    pub absolute_change_base: Decimal,
    /// The change with the flows taken out — the period's earnings.
    #[serde(with = "rust_decimal::serde::str")]
    pub delta_base: Decimal,
    /// Money committed by the end of the period: what was already there plus what was added.
    #[serde(with = "rust_decimal::serde::str")]
    pub invested_capital_base: Decimal,
    /// Modified Dietz denominator: capital at work, weighted by the time it was invested.
    #[serde(with = "rust_decimal::serde::str")]
    pub average_capital_base: Decimal,
}

impl PeriodSummary {
    /// An amount as a share of the capital that was at work — a fee rate, a tax rate, a turnover.
    /// `None` when no capital was committed: dividing by it would report an infinite rate.
    pub fn rate_of(&self, amount: Decimal) -> Option<Decimal> {
        if self.average_capital_base <= Decimal::ZERO {
            return None;
        }
        Some(amount / self.average_capital_base)
    }
}

/// Summarizes a value series. A window opens on what was there *before* it, so the first day's
/// own deposits are money paid in rather than an opening balance — at inception that makes the
/// opening zero instead of the first purchase. The first day's value is still what the capital
/// at work is measured from: money that arrived that day worked the whole window, which is the
/// same assumption [`dietz_capital`] makes about every later flow.
pub fn period_summary(series: &ValueSeries) -> PeriodSummary {
    let day_one_value = series.total_value_base.first().copied().unwrap_or_default();
    let day_one_flow = series.external_flow_base.first().copied().unwrap_or_default();
    let start_value_base = day_one_value - day_one_flow;
    let end_value_base = series.total_value_base.last().copied().unwrap_or_default();
    let net_flow_base: Decimal = series.external_flow_base.iter().copied().sum();

    let (from, to) = match (series.first_date(), series.last_date()) {
        (Some(from), Some(to)) => (from, to),
        _ => {
            return PeriodSummary {
                start_value_base,
                end_value_base,
                net_flow_base,
                absolute_change_base: Decimal::ZERO,
                delta_base: Decimal::ZERO,
                invested_capital_base: start_value_base,
                average_capital_base: start_value_base,
            };
        }
    };

    let flows = series
        .dates
        .iter()
        .copied()
        .zip(series.external_flow_base.iter().copied());

    PeriodSummary {
        absolute_change_base: end_value_base - start_value_base,
        delta_base: end_value_base - start_value_base - net_flow_base,
        invested_capital_base: start_value_base + net_flow_base,
        average_capital_base: dietz_capital(day_one_value, flows, from, to),
        start_value_base,
        end_value_base,
        net_flow_base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::normalize_currency;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 6, day).unwrap()
    }

    fn series(rows: &[(u32, Decimal, Decimal)]) -> ValueSeries {
        ValueSeries {
            base_currency: normalize_currency("EUR"),
            dates: rows.iter().map(|(day, _, _)| d(*day)).collect(),
            total_value_base: rows.iter().map(|(_, v, _)| *v).collect(),
            external_flow_base: rows.iter().map(|(_, _, f)| *f).collect(),
        }
    }

    /// 1000 on the 1st, 500 added on the 11th, 1700 on the 21st.
    /// Absolute change = 1700 - 1000 = 700; delta = 700 - 500 = 200.
    /// Invested capital = 1000 + 500 = 1500.
    /// Average capital = 1000 + 500 * (21 - 11) / (21 - 1) = 1000 + 250 = 1250.
    #[test]
    fn a_deposit_is_not_a_result() {
        let s = series(&[
            (1, dec!(1000), Decimal::ZERO),
            (11, dec!(1500), dec!(500)),
            (21, dec!(1700), Decimal::ZERO),
        ]);
        let summary = period_summary(&s);
        assert_eq!(summary.absolute_change_base, dec!(700));
        assert_eq!(summary.delta_base, dec!(200));
        assert_eq!(summary.invested_capital_base, dec!(1500));
        assert_eq!(summary.average_capital_base, dec!(1250));
    }

    /// A window that opens on the day the money arrived opens on nothing: 1000 paid in on the
    /// 1st, worth 1100 on the 21st. Opening = 1000 - 1000 = 0, so the whole 1000 is money paid
    /// in and the statement's change is 1100 - 0 = 1100. Earned is still 1100 - 0 - 1000 = 100,
    /// and the capital at work is still the 1000 that worked the period: it arrived on the
    /// opening day, so it is weighted whole.
    #[test]
    fn the_first_days_flow_is_money_paid_in_not_an_opening_balance() {
        let s = series(&[(1, dec!(1000), dec!(1000)), (21, dec!(1100), Decimal::ZERO)]);
        let summary = period_summary(&s);
        assert_eq!(summary.start_value_base, Decimal::ZERO);
        assert_eq!(summary.net_flow_base, dec!(1000));
        assert_eq!(summary.absolute_change_base, dec!(1100));
        assert_eq!(summary.delta_base, dec!(100));
        assert_eq!(summary.invested_capital_base, dec!(1000));
        assert_eq!(summary.average_capital_base, dec!(1000));
    }

    /// A window opened mid-life keeps its real opening balance: 1000 already there on the 1st
    /// with 200 paid in the same day is worth 1200 that evening, so the opening is
    /// 1200 - 200 = 1000 and the 200 counts as paid in. By the 21st it is 1300:
    /// change = 1300 - 1000 = 300, earned = 300 - 200 = 100.
    #[test]
    fn a_deposit_on_the_opening_day_still_counts_as_paid_in() {
        let s = series(&[(1, dec!(1200), dec!(200)), (21, dec!(1300), Decimal::ZERO)]);
        let summary = period_summary(&s);
        assert_eq!(summary.start_value_base, dec!(1000));
        assert_eq!(summary.net_flow_base, dec!(200));
        assert_eq!(summary.absolute_change_base, dec!(300));
        assert_eq!(summary.delta_base, dec!(100));
        assert_eq!(summary.invested_capital_base, dec!(1200));
        assert_eq!(summary.average_capital_base, dec!(1200));
    }

    /// A 12.50 fee against 1250 of capital at work is 1%.
    #[test]
    fn a_cost_rate_divides_by_the_capital_at_work() {
        let s = series(&[
            (1, dec!(1000), Decimal::ZERO),
            (11, dec!(1500), dec!(500)),
            (21, dec!(1700), Decimal::ZERO),
        ]);
        let summary = period_summary(&s);
        assert_eq!(summary.rate_of(dec!(12.50)), Some(dec!(0.01)));
    }

    /// Without capital there is no rate; reporting zero would claim the fee was free.
    #[test]
    fn an_empty_portfolio_has_no_cost_rate() {
        let s = series(&[
            (1, Decimal::ZERO, Decimal::ZERO),
            (21, Decimal::ZERO, Decimal::ZERO),
        ]);
        assert_eq!(period_summary(&s).rate_of(dec!(5)), None);
    }
}
