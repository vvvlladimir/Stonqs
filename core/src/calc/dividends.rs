use super::{Holdings, IncomeRecord};
use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Dividends rolled up over a group of payments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DividendSummary {
    pub payments: usize,
    /// Gross, before tax withholding.
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

impl DividendSummary {
    fn add(&mut self, d: &IncomeRecord) {
        self.payments += 1;
        self.gross_base += d.gross_base;
        self.taxes_base += d.taxes_base;
        self.fees_base += d.fees_base;
        self.net_base += d.net_base;
    }
}

/// Dividend payments out of an income ledger; the caller may hand over a whole
/// history ([`Holdings::income`]) or one window ([`super::income_between`]).
pub fn dividend_records(items: &[IncomeRecord]) -> impl Iterator<Item = &IncomeRecord> {
    items
        .iter()
        .filter(|r| r.kind == crate::model::TransactionKind::Dividend)
}

pub fn dividends_by_year(items: &[IncomeRecord]) -> BTreeMap<i32, DividendSummary> {
    group(items, |d| d.date.year())
}

/// Payments with no security are excluded here (there's no key) but still count
/// toward `dividends_by_year` and the total.
pub fn dividends_by_security(items: &[IncomeRecord]) -> BTreeMap<String, DividendSummary> {
    let mut out: BTreeMap<String, DividendSummary> = BTreeMap::new();
    for d in dividend_records(items) {
        if let Some(sid) = &d.security_id {
            out.entry(sid.clone()).or_default().add(d);
        }
    }
    out
}

pub fn dividends_total(items: &[IncomeRecord]) -> DividendSummary {
    let mut total = DividendSummary::default();
    for d in dividend_records(items) {
        total.add(d);
    }
    total
}

/// Yield on cost: cumulative dividends divided by an open position's historical cost basis;
/// `None` means the position is closed or has zero cost.
pub fn yield_on_cost(holdings: &Holdings, security_id: &str) -> Option<Decimal> {
    let position = holdings.positions.get(security_id)?;
    if position.cost_basis_base.is_zero() {
        return None;
    }
    Some(position.dividends_base / position.cost_basis_base)
}

fn group<K: Ord, F: Fn(&IncomeRecord) -> K>(items: &[IncomeRecord], key: F) -> BTreeMap<K, DividendSummary> {
    let mut out: BTreeMap<K, DividendSummary> = BTreeMap::new();
    for d in dividend_records(items) {
        out.entry(key(d)).or_default().add(d);
    }
    out
}

/// How often an instrument pays, read off the gaps between its own payments rather than
/// declared anywhere: a broker export states no schedule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DividendFrequency {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    /// Gaps longer than a year, or a single special payment: no schedule to name.
    Irregular,
    /// Fewer than two payments — there is no gap to measure yet.
    #[default]
    Unknown,
}

/// What the payment history of one instrument says about it: how much, how often, how recently.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DividendProfile {
    pub payments: usize,
    pub first_payment: Option<NaiveDate>,
    pub last_payment: Option<NaiveDate>,
    pub frequency: DividendFrequency,
    /// Lifetime totals, so a yield on cost can be read off the same struct.
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
    /// Net paid in the year ending at `as_of` — the numerator of a yearly yield. A payer that
    /// stopped shows zero here while `net_base` keeps its history.
    #[serde(with = "rust_decimal::serde::str")]
    pub trailing_year_base: Decimal,
    /// Typical gap between payments in days; `None` with fewer than two.
    pub median_gap_days: Option<i64>,
}

/// Payment facts for one instrument's records. The caller filters by security, because a
/// profile of "every dividend the portfolio received" is not a schedule of anything.
pub fn dividend_profile(items: &[IncomeRecord], as_of: NaiveDate) -> DividendProfile {
    let mut profile = DividendProfile::default();
    // Two accounts paid on one ex-date is one payment of the instrument, not two gaps of zero.
    let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();
    let year_ago = as_of - Duration::days(365);

    for d in dividend_records(items) {
        profile.payments += 1;
        profile.gross_base += d.gross_base;
        profile.net_base += d.net_base;
        if d.date > year_ago && d.date <= as_of {
            profile.trailing_year_base += d.net_base;
        }
        dates.insert(d.date);
    }

    profile.first_payment = dates.first().copied();
    profile.last_payment = dates.last().copied();
    profile.median_gap_days = median_gap(&dates);
    profile.frequency = DividendFrequency::of_gap(profile.median_gap_days);
    profile
}

impl DividendFrequency {
    /// The schedule a typical gap between payments names.
    pub(crate) fn of_gap(median_gap_days: Option<i64>) -> Self {
        match median_gap_days {
            // Buckets are wide because real payers drift: a "quarterly" one pays at 89 and 93 days,
            // and a median absorbs a single skipped quarter without renaming the schedule.
            Some(gap) if gap <= 45 => DividendFrequency::Monthly,
            Some(gap) if gap <= 135 => DividendFrequency::Quarterly,
            Some(gap) if gap <= 250 => DividendFrequency::SemiAnnual,
            Some(gap) if gap <= 450 => DividendFrequency::Annual,
            Some(_) => DividendFrequency::Irregular,
            None => DividendFrequency::Unknown,
        }
    }

    /// Payments a year on this schedule; `None` when there is no schedule.
    pub(crate) fn per_year(self) -> Option<usize> {
        match self {
            DividendFrequency::Monthly => Some(12),
            DividendFrequency::Quarterly => Some(4),
            DividendFrequency::SemiAnnual => Some(2),
            DividendFrequency::Annual => Some(1),
            DividendFrequency::Irregular | DividendFrequency::Unknown => None,
        }
    }
}

/// One profile per paying instrument. Payments without a security have no schedule to belong to.
pub fn dividend_profiles(items: &[IncomeRecord], as_of: NaiveDate) -> BTreeMap<String, DividendProfile> {
    let mut by_security: BTreeMap<String, Vec<IncomeRecord>> = BTreeMap::new();
    for d in dividend_records(items) {
        if let Some(sid) = &d.security_id {
            by_security.entry(sid.clone()).or_default().push(d.clone());
        }
    }
    by_security
        .into_iter()
        .map(|(sid, records)| (sid, dividend_profile(&records, as_of)))
        .collect()
}

/// Middle gap between consecutive payment dates; an even count averages the two middles.
pub(crate) fn median_gap(dates: &BTreeSet<NaiveDate>) -> Option<i64> {
    let mut gaps: Vec<i64> = dates
        .iter()
        .zip(dates.iter().skip(1))
        .map(|(a, b)| (*b - *a).num_days())
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_unstable();
    let middle = gaps.len() / 2;
    Some(if gaps.len().is_multiple_of(2) {
        (gaps[middle - 1] + gaps[middle]) / 2
    } else {
        gaps[middle]
    })
}

/// A yearly dividend yield: the last year's payments over a value. `None` when there is no
/// value to divide by — a closed position pays no yield, it does not pay an infinite one.
pub fn dividend_yield(trailing_year_base: Decimal, value_base: Decimal) -> Option<Decimal> {
    if value_base <= Decimal::ZERO {
        return None;
    }
    Some(trailing_year_base / value_base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TransactionKind;
    use crate::money::normalize_currency;
    use rust_decimal_macros::dec;

    fn payment(date: (i32, u32, u32), net: Decimal) -> IncomeRecord {
        IncomeRecord {
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            account_id: "acc".into(),
            security_id: Some("VWCE".into()),
            kind: TransactionKind::Dividend,
            gross_base: net,
            taxes_base: Decimal::ZERO,
            fees_base: Decimal::ZERO,
            net_base: net,
            currency: normalize_currency("EUR"),
            gross_in_currency: net,
        }
    }

    /// Gaps of 91, 92 and 92 days: the median is 92, which is a quarter.
    /// The last year (2024-06-13 .. 2025-06-13) holds the last three: 11 + 12 + 13 = 36.
    #[test]
    fn four_payments_a_year_apart_read_as_quarterly() {
        let records = [
            payment((2024, 3, 15), dec!(10)),
            payment((2024, 6, 14), dec!(11)),
            payment((2024, 9, 14), dec!(12)),
            payment((2024, 12, 15), dec!(13)),
        ];
        let profile = dividend_profile(&records, NaiveDate::from_ymd_opt(2025, 6, 13).unwrap());

        assert_eq!(profile.payments, 4);
        assert_eq!(profile.median_gap_days, Some(92));
        assert_eq!(profile.frequency, DividendFrequency::Quarterly);
        assert_eq!(profile.net_base, dec!(46));
        assert_eq!(profile.trailing_year_base, dec!(36));
        assert_eq!(
            profile.last_payment,
            Some(NaiveDate::from_ymd_opt(2024, 12, 15).unwrap())
        );
    }

    /// One payment leaves no gap, so the schedule is unknown rather than annual.
    #[test]
    fn a_single_payment_names_no_schedule() {
        let profile = dividend_profile(
            &[payment((2024, 3, 15), dec!(10))],
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        );

        assert_eq!(profile.frequency, DividendFrequency::Unknown);
        assert_eq!(profile.median_gap_days, None);
    }

    /// 36 paid over a position worth 1200 is a 3% yield.
    #[test]
    fn the_yield_divides_the_year_by_the_value() {
        assert_eq!(dividend_yield(dec!(36), dec!(1200)), Some(dec!(0.03)));
        assert_eq!(dividend_yield(dec!(36), Decimal::ZERO), None);
    }
}
