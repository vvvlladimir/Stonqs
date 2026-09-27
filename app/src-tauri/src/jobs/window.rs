//! Where each subject's refresh begins.

use super::RefreshMode;
use chrono::{Duration, NaiveDate};
use sq_core::prelude::DateRange;
use sq_core::storage::Store;

/// The mode's window, widened back to the first operation when what is stored does not reach it:
/// a catch-up starts where the series ends, so an older hole would never close by itself.
pub(super) fn start_from(
    mode: RefreshMode,
    known: Option<NaiveDate>,
    today: NaiveDate,
    stored_from: Option<NaiveDate>,
    needed_from: Option<NaiveDate>,
) -> NaiveDate {
    let start = since(mode, known, today);
    match needed_from {
        Some(need) if need < start && stored_from.is_none_or(|from| need < from) => need,
        _ => start,
    }
}

/// Five years back, or the last known day minus a three-day correction window.
pub(super) fn since(mode: RefreshMode, known: Option<NaiveDate>, today: NaiveDate) -> NaiveDate {
    let full = today - Duration::days(365 * 5);
    match (mode, known) {
        (RefreshMode::Full, _) | (_, None) => full,
        (_, Some(last)) => (last - Duration::days(3)).max(full),
    }
}

/// The start of what was asked for a security; `None` unless both quote and event coverage hold.
pub(super) fn asked_from(store: &Store, security_id: &str) -> Option<NaiveDate> {
    match (
        store.quote_coverage(security_id),
        store.event_coverage(security_id),
    ) {
        (Ok(Some(quotes)), Ok(Some(events))) => Some(quotes.from.max(events.from)),
        _ => None,
    }
}

/// A pair's first stored rate. FX keeps no coverage table, so the rates are the record.
pub(super) fn stored_rates_from(
    store: &Store,
    currency: &str,
    base: &str,
    today: NaiveDate,
) -> Option<NaiveDate> {
    store
        .rate_series(currency, base, today)
        .ok()
        .and_then(|series| series.keys().next().copied())
}

/// A series covering under a quarter of a holding of 90+ days: the shape a ticker on the wrong
/// venue leaves, answered with today's price alone.
pub fn sparse_history(held_from: Option<NaiveDate>, span: Option<DateRange>, today: NaiveDate) -> bool {
    let Some(held) = held_from else {
        return false;
    };
    let wanted = (today - held).num_days();
    if wanted < 90 {
        return false;
    }
    match span {
        None => true,
        Some(span) => (span.to - span.from).num_days() * 4 < wanted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Importing five years into a database holding this year's must widen the window back to
    /// the first operation.
    #[test]
    fn a_window_widens_back_to_the_first_operation_it_does_not_reach() {
        let today = d(2026, 9, 22);
        let stored = Some(d(2025, 1, 2));
        let first = Some(d(2019, 3, 4));

        assert_eq!(
            start_from(RefreshMode::CatchUp, Some(d(2026, 9, 19)), today, stored, first),
            d(2019, 3, 4)
        );
        // Already reaching back far enough: the three-day correction window stands.
        assert_eq!(
            start_from(
                RefreshMode::CatchUp,
                Some(d(2026, 9, 19)),
                today,
                Some(d(2018, 1, 1)),
                first
            ),
            d(2026, 9, 16)
        );
        // Nothing stored and nothing held: the five-year default.
        assert_eq!(
            start_from(RefreshMode::Missing, None, today, None, None),
            today - Duration::days(365 * 5)
        );
    }

    /// A short holding and a late listing are not a wrong venue.
    #[test]
    fn a_series_covering_a_sliver_of_the_holding_is_sparse() {
        let today = d(2026, 9, 22);
        let held = Some(d(2021, 9, 1));
        let one_day = |day| Some(DateRange::new(day, day));

        assert!(sparse_history(held, None, today));
        assert!(sparse_history(held, one_day(d(2026, 9, 22)), today));
        // Two of five years held: a later listing, not a broken ticker.
        assert!(!sparse_history(
            held,
            Some(DateRange::new(d(2024, 9, 1), today)),
            today
        ));
        // Bought last week: nothing to say yet.
        assert!(!sparse_history(Some(d(2026, 9, 15)), one_day(today), today));
        // Never traded: no holding to measure against.
        assert!(!sparse_history(None, None, today));
    }
}
