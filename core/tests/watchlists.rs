//! A watched instrument's figures, read off its own closes. See ADR-0035.

mod support;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::{alert_status, instrument_move, nearest_level};
use sq_core::market::PricePoint;
use sq_core::model::{SecurityAlert, SecurityEvent};
use std::collections::BTreeMap;
use support::{FakeRates, d};

const SAP: &str = "sec-sap";

fn closes(currency: &str, points: &[(NaiveDate, Decimal)]) -> BTreeMap<NaiveDate, PricePoint> {
    points
        .iter()
        .map(|(date, close)| (*date, PricePoint::new(*close, currency)))
        .collect()
}

/// Closes in EUR: Jan 2 64 · May 31 50 · Jun 3 48 · Jun 4 55 · Jun 5 60 · Jun 6 57.
/// Period Jun 1 – Jun 7, read on Jun 7 (no close that day, so Jun 6 is the price).
/// Today:  57 / 60 − 1 = −0.05.
/// Period: starts at the close in force on Jun 1, May 31's 50 → 57 / 50 − 1 = 0.14.
/// Range:  closes from May 31 on — 50, 48, 55, 60, 57 → low 48, high 60;
///         position (57 − 48) / (60 − 48) = 9 / 12 = 0.75.
/// High:   Jan 2's 64 → 57 / 64 − 1 = −0.109375.
/// Reported dividends inside the year (Jun 7 2023, Jun 7 2024]: 0.50 on Dec 1 + 0.50 on Mar 1
///         = 1.00 per share; Jun 1 2023 is outside, a USD one is another currency.
///         Yield 1.00 / 57 = 0.017543859… ≈ 0.017544. Last reported: Mar 1.
#[test]
fn a_watched_instrument_is_read_off_its_own_closes() {
    let prices = closes(
        "EUR",
        &[
            (d(2024, 1, 2), dec!(64)),
            (d(2024, 5, 31), dec!(50)),
            (d(2024, 6, 3), dec!(48)),
            (d(2024, 6, 4), dec!(55)),
            (d(2024, 6, 5), dec!(60)),
            (d(2024, 6, 6), dec!(57)),
        ],
    );
    let events = [
        SecurityEvent::dividend(SAP, d(2023, 6, 1), dec!(0.40), "EUR", "yahoo"),
        SecurityEvent::dividend(SAP, d(2023, 12, 1), dec!(0.50), "EUR", "yahoo"),
        SecurityEvent::dividend(SAP, d(2024, 3, 1), dec!(0.50), "EUR", "yahoo"),
        SecurityEvent::dividend(SAP, d(2024, 4, 1), dec!(9), "USD", "yahoo"),
        SecurityEvent::note(SAP, d(2024, 5, 1), "capital markets day"),
    ];

    let m = instrument_move(&prices, &events, d(2024, 6, 1), d(2024, 6, 7));

    assert_eq!(m.currency.as_deref(), Some("EUR"));
    assert_eq!((m.price, m.price_date), (Some(dec!(57)), Some(d(2024, 6, 6))));
    assert_eq!(m.previous_price, Some(dec!(60)));
    assert_eq!(m.day_change, Some(dec!(-0.05)));
    assert_eq!(
        (m.period_start, m.start_price),
        (Some(d(2024, 5, 31)), Some(dec!(50)))
    );
    assert_eq!(m.period_return, Some(dec!(0.14)));
    assert_eq!((m.low, m.high), (Some(dec!(48)), Some(dec!(60))));
    assert_eq!(m.range_position, Some(dec!(0.75)));
    assert_eq!((m.ath_price, m.ath_date), (Some(dec!(64)), Some(d(2024, 1, 2))));
    assert_eq!(m.ath_distance, Some(dec!(-0.109375)));
    assert_eq!(m.dividend_year, Some(dec!(1.00)));
    assert_eq!(m.dividend_yield.map(|y| y.round_dp(6)), Some(dec!(0.017544)));
    assert_eq!(m.dividend_last, Some(d(2024, 3, 1)));
}

/// Listed Jun 4 at 55, period from Jun 1: there is no close in force on Jun 1, so the move
/// starts at the first one — 57 / 55 − 1 = 0.036363… ≈ 0.036364, and the range is 55 – 57.
/// A USD close from an earlier listing is another currency and is not part of it.
#[test]
fn an_instrument_younger_than_the_period_is_measured_from_its_first_close() {
    let mut prices = closes("EUR", &[(d(2024, 6, 4), dec!(55)), (d(2024, 6, 6), dec!(57))]);
    prices.insert(d(2024, 5, 20), PricePoint::new(dec!(70), "USD"));

    let m = instrument_move(&prices, &[], d(2024, 6, 1), d(2024, 6, 7));

    assert_eq!(m.period_start, Some(d(2024, 6, 4)));
    assert_eq!(m.period_return.map(|r| r.round_dp(6)), Some(dec!(0.036364)));
    assert_eq!((m.low, m.high), (Some(dec!(55)), Some(dec!(57))));
    assert_eq!(m.dividend_year, None);
}

/// Nothing on file before the day asked about: every figure is absent, not zero.
#[test]
fn an_instrument_without_closes_has_no_figures() {
    let prices = closes("EUR", &[(d(2024, 6, 10), dec!(55))]);
    let m = instrument_move(&prices, &[], d(2024, 6, 1), d(2024, 6, 7));
    assert_eq!(m.price, None);
    assert_eq!(m.period_return, None);
}

/// Price 57 €. Levels: 61.56 → 61.56 / 57 − 1 = +0.08; 55.29 → 55.29 / 57 − 1 = −0.03.
/// A date rule has no distance. Nearest by size: −0.03, the level below.
#[test]
fn the_nearest_level_is_the_smallest_distance_either_way() {
    let prices = closes("EUR", &[(d(2024, 6, 6), dec!(57))]);
    let rates = FakeRates::new();
    let rules = [
        SecurityAlert::price(SAP, dec!(61.56), "EUR", d(2024, 6, 1)),
        SecurityAlert::price(SAP, dec!(55.29), "EUR", d(2024, 6, 1)),
        SecurityAlert::date_reached(SAP, d(2024, 7, 1), d(2024, 6, 1)),
    ];
    let statuses: Vec<_> = rules
        .iter()
        .map(|rule| alert_status(rule, &prices, &rates, d(2024, 6, 7)).unwrap())
        .collect();

    let nearest = nearest_level(rules.iter().zip(&statuses)).unwrap();

    assert_eq!(nearest.alert_id, rules[1].id);
    assert_eq!(nearest.level, dec!(55.29));
    assert_eq!(nearest.distance, dec!(-0.03));
}
