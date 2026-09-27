use super::*;
use crate::error::{Error, Result};
use crate::market::Quote;
use crate::market::{DateRange, FetchPolicy, QuoteProvider};
use crate::model::Security;
use crate::model::SecurityKind;
use crate::storage::Store;
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use std::sync::Mutex;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// A source that is down, or that closes every day at `close` in `currency`.
struct Fixed {
    id: &'static str,
    down: bool,
    close: rust_decimal::Decimal,
    currency: &'static str,
    calls: std::sync::Arc<Mutex<usize>>,
}

impl Fixed {
    fn up(id: &'static str, close: rust_decimal::Decimal, currency: &'static str) -> Box<Self> {
        Box::new(Fixed {
            id,
            down: false,
            close,
            currency,
            calls: Default::default(),
        })
    }
    fn down(id: &'static str) -> Box<Self> {
        Box::new(Fixed {
            id,
            down: true,
            close: dec!(0),
            currency: "USD",
            calls: Default::default(),
        })
    }
}

impl QuoteProvider for Fixed {
    fn id(&self) -> &'static str {
        self.id
    }
    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        *self.calls.lock().unwrap() += 1;
        if self.down {
            return Err(Error::Unavailable("503".into()));
        }
        Ok(range
            .from
            .iter_days()
            .take_while(|day| *day <= range.to)
            .map(|date| Quote {
                security_id: security.id.clone(),
                date,
                close: self.close,
                currency: self.currency.into(),
                source: self.id.into(),
            })
            .collect())
    }
}

fn listed(store: &Store, symbol: &str) -> Security {
    let sec = Security::new(symbol, symbol, "USD", SecurityKind::Stock).with_source("primary", symbol);
    store.save_security(&sec).unwrap();
    store.set_security_symbol(&sec.id, "backup", symbol).unwrap();
    sec
}

fn chain(backup: Box<Fixed>) -> MarketDataService {
    MarketDataService::new()
        .with_policy(FetchPolicy::none())
        .with(Fixed::down("primary"))
        .with(backup)
}

#[test]
fn a_fallback_fills_the_gap_without_claiming_coverage() {
    let store = Store::open_in_memory().unwrap();
    let sec = listed(&store, "ACME");
    let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));

    // 5 days asked, fallback answers from 14 days earlier: 5 + 14 = 19 rows, all new.
    let saved = chain(Fixed::up("backup", dec!(10), "USD"))
        .ensure_history(&store, &sec, week)
        .unwrap();
    assert_eq!(saved, 19);
    assert_eq!(store.quotes_in_range(&sec.id, week).unwrap()[0].source, "backup");
    assert_eq!(store.quote_coverage(&sec.id).unwrap(), None);
}

#[test]
fn a_fallback_that_disagrees_with_the_stored_series_is_refused() {
    let store = Store::open_in_memory().unwrap();
    let sec = listed(&store, "ACME");
    let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
    store
        .save_quotes(&[Quote {
            security_id: sec.id.clone(),
            date: d(2024, 5, 31),
            close: dec!(10),
            currency: "USD".into(),
            source: "primary".into(),
        }])
        .unwrap();

    // Stored 10, fallback 1000 on 05-31: ratio 100, far outside 2%.
    let err = chain(Fixed::up("backup", dec!(1000), "USD")).ensure_history(&store, &sec, week);
    assert!(matches!(err, Err(Error::Unavailable(_))));
    assert_eq!(store.quotes_in_range(&sec.id, week).unwrap(), vec![]);
}

#[test]
fn a_source_without_the_instrument_s_symbol_is_not_asked() {
    let store = Store::open_in_memory().unwrap();
    let sec = Security::new("SOLO", "Solo", "USD", SecurityKind::Stock).with_source("primary", "SOLO");
    store.save_security(&sec).unwrap();
    let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
    assert!(
        chain(Fixed::up("backup", dec!(10), "USD"))
            .ensure_history(&store, &sec, week)
            .is_err()
    );
}

/// An instrument whose own source has published through 06-05 and whose latest close comes
/// from `backup`.
fn with_latest(store: &Store, close: rust_decimal::Decimal) -> Security {
    let sec = listed(store, "FUND");
    store.set_latest_source(&sec.id, Some("backup")).unwrap();
    let history: Vec<Quote> = d(2024, 6, 3)
        .iter_days()
        .take(3)
        .map(|date| Quote {
            security_id: sec.id.clone(),
            date,
            close,
            currency: "USD".into(),
            source: "primary".into(),
        })
        .collect();
    store.save_quotes(&history).unwrap();
    sec
}

#[test]
fn the_latest_source_fills_only_the_tail_the_own_source_has_not_published() {
    let store = Store::open_in_memory().unwrap();
    let sec = with_latest(&store, dec!(10));
    let svc = chain(Fixed::up("backup", dec!(10.1), "USD"));

    // Stored 06-03..06-05; asked through 06-07 → only 06-06 and 06-07 are new (10.1 vs 10 is
    // a 1% ratio, inside the 2% guard). The three stored days keep the own source's close.
    let saved = svc.ensure_latest(&store, &sec, d(2024, 6, 7)).unwrap();
    assert_eq!(saved, 2);
    let week = store
        .quotes_in_range(&sec.id, DateRange::new(d(2024, 6, 3), d(2024, 6, 7)))
        .unwrap();
    let sources: Vec<&str> = week.iter().map(|q| q.source.as_str()).collect();
    assert_eq!(sources, ["primary", "primary", "primary", "backup", "backup"]);
    assert_eq!(
        store.quote_coverage(&sec.id).unwrap(),
        None,
        "no coverage is claimed"
    );

    // Up to date: nothing is asked at all.
    assert_eq!(svc.ensure_latest(&store, &sec, d(2024, 6, 7)).unwrap(), 0);
}

#[test]
fn a_latest_close_that_disagrees_with_the_history_is_refused() {
    let store = Store::open_in_memory().unwrap();
    let sec = with_latest(&store, dec!(10));

    // 10.5 against 10 is a 5% ratio: another series, not a fresher close of this one.
    let err = chain(Fixed::up("backup", dec!(10.5), "USD")).ensure_latest(&store, &sec, d(2024, 6, 7));
    assert!(matches!(err, Err(Error::BadProviderData { .. })), "{err:?}");
    assert_eq!(store.latest_quote_date(&sec.id).unwrap(), Some(d(2024, 6, 5)));
}

#[test]
fn a_latest_source_that_is_switched_off_asks_nothing_and_fails_nothing() {
    let store = Store::open_in_memory().unwrap();
    let sec = with_latest(&store, dec!(10));
    // Only the own source is registered: `backup` is off in the settings.
    let service = MarketDataService::new()
        .with_policy(FetchPolicy::none())
        .with(Fixed::down("primary"));
    assert_eq!(service.ensure_latest(&store, &sec, d(2024, 6, 7)).unwrap(), 0);
}

#[test]
fn without_a_latest_source_nothing_is_asked() {
    let store = Store::open_in_memory().unwrap();
    let sec = listed(&store, "ACME");
    let backup = Fixed::up("backup", dec!(10), "USD");
    let calls = backup.calls.clone();
    assert_eq!(
        chain(backup).ensure_latest(&store, &sec, d(2024, 6, 7)).unwrap(),
        0
    );
    assert_eq!(*calls.lock().unwrap(), 0);
}

#[test]
fn a_source_that_keeps_failing_rests_for_the_rest_of_the_refresh() {
    let store = Store::open_in_memory().unwrap();
    let primary = Fixed::down("primary");
    let calls = primary.calls.clone();
    let svc = MarketDataService::new()
        .with_policy(FetchPolicy::none())
        .with(primary);
    let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
    for symbol in ["A", "B", "C", "D", "E"] {
        let _ = svc.ensure_history(&store, &listed(&store, symbol), week);
    }
    assert_eq!(*calls.lock().unwrap(), RESTS_AFTER as usize);
}
