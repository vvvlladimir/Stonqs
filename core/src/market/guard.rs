//! Whether a fallback source's quotes may join an instrument's stored series.

use super::Quote;
use crate::error::{Error, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// Two sources closing the same day further apart than this are not quoting the same series: one
/// is split- or dividend-adjusted differently, or quotes in pence rather than pounds.
const TOLERANCE: Decimal = dec!(0.02);

/// Refuses `incoming` unless it is in `currency` and, on the days both hold, closes within
/// `TOLERANCE` of `stored` (median ratio). With no day in common there is nothing to disagree with.
pub(crate) fn check(
    source: &'static str,
    currency: &str,
    stored: &[Quote],
    incoming: &[Quote],
) -> Result<()> {
    if let Some(q) = incoming.iter().find(|q| q.currency != currency) {
        return Err(Error::BadProviderData {
            provider: source,
            detail: format!("quotes in {} where the series is in {currency}", q.currency),
        });
    }
    let stored: BTreeMap<_, _> = stored.iter().map(|q| (q.date, q.close)).collect();
    let mut ratios: Vec<Decimal> = incoming
        .iter()
        .filter_map(|q| stored.get(&q.date).filter(|s| !s.is_zero()).map(|s| q.close / s))
        .collect();
    if ratios.is_empty() {
        return Ok(());
    }
    ratios.sort_unstable();
    let median = ratios[ratios.len() / 2];
    if (median - Decimal::ONE).abs() > TOLERANCE {
        return Err(Error::BadProviderData {
            provider: source,
            detail: format!(
                "series disagrees with the stored one (median ratio {})",
                median.round_dp(4)
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn q(day: u32, close: Decimal, currency: &str) -> Quote {
        Quote {
            security_id: "s".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, day).unwrap(),
            close,
            currency: currency.into(),
            source: "x".into(),
        }
    }

    #[test]
    fn a_close_within_a_percent_on_common_days_joins() {
        // stored 100, 102; incoming 100.5, 101.5 -> ratios 1.005, 0.9951 -> median 1.005, inside 2%.
        let stored = [q(3, dec!(100), "USD"), q(4, dec!(102), "USD")];
        let incoming = [
            q(3, dec!(100.5), "USD"),
            q(4, dec!(101.5), "USD"),
            q(5, dec!(103), "USD"),
        ];
        assert!(check("x", "USD", &stored, &incoming).is_ok());
    }

    #[test]
    fn pence_against_pounds_is_refused() {
        // 5234 / 52.34 = 100: the same close a hundred times over.
        let stored = [q(3, dec!(52.34), "GBP")];
        let incoming = [q(3, dec!(5234), "GBP")];
        assert!(check("x", "GBP", &stored, &incoming).is_err());
    }

    #[test]
    fn an_unadjusted_split_is_refused() {
        // Stored is split-adjusted 1:10 (12.00), the fallback still shows 120.00: ratio 10.
        let stored = [q(3, dec!(12), "USD"), q(4, dec!(12.1), "USD")];
        let incoming = [q(3, dec!(120), "USD"), q(4, dec!(121), "USD")];
        assert!(check("x", "USD", &stored, &incoming).is_err());
    }

    #[test]
    fn another_currency_is_refused_even_without_common_days() {
        assert!(check("x", "EUR", &[], &[q(3, dec!(10), "USD")]).is_err());
    }

    #[test]
    fn nothing_in_common_is_nothing_to_disagree_with() {
        assert!(check("x", "USD", &[q(3, dec!(10), "USD")], &[q(5, dec!(99), "USD")]).is_ok());
    }
}
