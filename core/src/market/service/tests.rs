use super::*;
use crate::error::Result;
use crate::market::Quote;
use crate::market::{DateRange, QuoteProvider};
use crate::model::Security;
use crate::model::SecurityKind;
use crate::storage::Store;
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use std::sync::Mutex;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn no_coverage_means_fetch_everything() {
    let want = DateRange::new(d(2024, 1, 1), d(2024, 3, 1));
    assert_eq!(missing_ranges(None, want), vec![want]);
}

#[test]
fn fully_covered_means_no_fetch() {
    let covered = DateRange::new(d(2024, 1, 1), d(2024, 12, 31));
    let want = DateRange::new(d(2024, 3, 1), d(2024, 4, 1));
    assert!(missing_ranges(Some(covered), want).is_empty());
}

#[test]
fn fetches_only_head_and_tail() {
    let covered = DateRange::new(d(2024, 2, 1), d(2024, 2, 29));
    let want = DateRange::new(d(2024, 1, 15), d(2024, 3, 10));
    let gaps = missing_ranges(Some(covered), want);
    assert_eq!(
        gaps,
        vec![
            DateRange::new(d(2024, 1, 15), d(2024, 1, 31)),
            DateRange::new(d(2024, 3, 1), d(2024, 3, 10)),
        ]
    );
}

struct CountingProvider {
    calls: Mutex<Vec<DateRange>>,
}

impl QuoteProvider for CountingProvider {
    fn id(&self) -> &'static str {
        "counting"
    }
    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        self.calls.lock().unwrap().push(range);
        Ok(vec![Quote {
            security_id: security.id.clone(),
            date: range.from,
            close: dec!(100),
            currency: security.currency.clone(),
            source: "counting".into(),
        }])
    }
}

#[test]
fn second_request_for_same_range_does_not_hit_provider() {
    let store = Store::open_in_memory().unwrap();
    let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("counting", "test");
    store.save_security(&sec).unwrap();

    let mut svc = MarketDataService::new();
    svc.register(Box::new(CountingProvider {
        calls: Mutex::new(Vec::new()),
    }));

    let range = DateRange::new(d(2024, 1, 1), d(2024, 1, 31));
    svc.ensure_history(&store, &sec, range).unwrap();
    svc.ensure_history(&store, &sec, range).unwrap();

    let provider = svc.provider("counting").unwrap();
    assert_eq!(provider.id(), "counting");
    let saved = store.quotes_in_range(&sec.id, range).unwrap();
    assert_eq!(saved.len(), 1);
}

struct ReportingProvider;

impl QuoteProvider for ReportingProvider {
    fn id(&self) -> &'static str {
        "reporting"
    }
    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        Ok(self.fetch_history(security, range)?.quotes)
    }
    fn fetch_history(&self, security: &Security, range: DateRange) -> Result<crate::market::History> {
        Ok(crate::market::History {
            quotes: Vec::new(),
            events: vec![crate::model::SecurityEvent::split(
                &security.id,
                range.from,
                dec!(1),
                dec!(4),
                "reporting",
            )],
        })
    }
}

/// Quotes fetched before events existed leave no event coverage, so the whole range is
/// asked again once and the events that come back are saved.
#[test]
fn a_range_without_event_coverage_is_fetched_again() {
    let store = Store::open_in_memory().unwrap();
    let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("reporting", "test");
    store.save_security(&sec).unwrap();
    let range = DateRange::new(d(2024, 1, 1), d(2024, 1, 31));
    store.extend_quote_coverage(&sec.id, range).unwrap();

    let svc = MarketDataService::new().with(Box::new(ReportingProvider));
    svc.ensure_history(&store, &sec, range).unwrap();

    assert_eq!(store.event_coverage(&sec.id).unwrap(), Some(range));
    assert_eq!(store.security_events_for(&sec.id).unwrap().len(), 1);
    assert!(missing_ranges(overlap(Some(range), Some(range)), range).is_empty());
}

#[test]
fn the_unsettled_day_is_asked_again() {
    let store = Store::open_in_memory().unwrap();
    let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("counting", "test");
    store.save_security(&sec).unwrap();

    let svc = MarketDataService::new().with(Box::new(CountingProvider {
        calls: Mutex::new(Vec::new()),
    }));

    let range = DateRange::new(d(2024, 1, 10), d(2024, 1, 15));
    let settled = d(2024, 1, 14);
    svc.ensure_history_through(&store, &sec, range, settled).unwrap();

    assert_eq!(
        store.quote_coverage(&sec.id).unwrap(),
        Some(DateRange::new(d(2024, 1, 10), settled))
    );

    assert_eq!(
        missing_ranges(store.quote_coverage(&sec.id).unwrap(), range),
        vec![DateRange::new(d(2024, 1, 15), d(2024, 1, 15))]
    );
}
