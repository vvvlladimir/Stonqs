//! Price triggers and date rules: which closes cross the level, and where the price stands.
//! See ADR-0034.

mod support;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::Error;
use sq_core::calc::{AlertCheck, alert_status, check_alert, crossing_close};
use sq_core::market::PricePoint;
use sq_core::model::{AlertDirection, AlertSide, CrossingDirection, SecurityAlert};
use std::collections::BTreeMap;
use support::{FakeRates, d};

const AAPL: &str = "sec-aapl";

fn closes(currency: &str, points: &[(NaiveDate, Decimal)]) -> BTreeMap<NaiveDate, PricePoint> {
    points
        .iter()
        .map(|(date, close)| (*date, PricePoint::new(*close, currency)))
        .collect()
}

fn apply(alert: &mut SecurityAlert, check: &AlertCheck) {
    alert.side = check.side;
    alert.checked_through = check.checked_through;
}

/// A 180 € level on a USD listing, created Jun 1, converted at 0.93.
/// May 31: 190 USD × 0.93 = 176.70 € — below; where the trigger starts, not a crossing.
/// Jun 3:  195 USD × 0.93 = 181.35 € — above: crossed up.
/// Jun 4:  200 USD × 0.93 = 186.00 € — still above.
/// Jun 5:  190 USD × 0.93 = 176.70 € — below: crossed down.
/// Checked on Jun 6: two crossings, bookmark Below as of Jun 5.
#[test]
fn every_change_of_side_is_a_crossing_either_way() {
    let alert = SecurityAlert::price(AAPL, dec!(180), "EUR", d(2024, 6, 1));
    let prices = closes(
        "USD",
        &[
            (d(2024, 5, 31), dec!(190)),
            (d(2024, 6, 3), dec!(195)),
            (d(2024, 6, 4), dec!(200)),
            (d(2024, 6, 5), dec!(190)),
        ],
    );
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 5, 1), dec!(0.93));

    let check = check_alert(&alert, &prices, &rates, d(2024, 6, 6)).unwrap();

    let log: Vec<_> = check
        .crossings
        .iter()
        .map(|c| (c.date, c.direction, c.price))
        .collect();
    assert_eq!(
        log,
        [
            (d(2024, 6, 3), CrossingDirection::Up, Some(dec!(181.35))),
            (d(2024, 6, 5), CrossingDirection::Down, Some(dec!(176.70))),
        ]
    );
    assert_eq!(check.crossings[0].level, Some(dec!(180)));
    assert_eq!(check.side, Some(AlertSide::Below));
    assert_eq!(check.checked_through, Some(d(2024, 6, 5)));
}

/// Checking again with the same closes logs nothing. When the Jun 5 close is rewritten to
/// 200 USD (186.00 €, above), the check re-reads that day and logs one crossing up.
#[test]
fn a_second_check_logs_only_what_changed() {
    let mut alert = SecurityAlert::price(AAPL, dec!(100), "USD", d(2024, 6, 1));
    let mut prices = closes("USD", &[(d(2024, 6, 1), dec!(95)), (d(2024, 6, 5), dec!(98))]);
    let rates = FakeRates::new();

    let first = check_alert(&alert, &prices, &rates, d(2024, 6, 5)).unwrap();
    assert!(first.crossings.is_empty());
    apply(&mut alert, &first);

    let again = check_alert(&alert, &prices, &rates, d(2024, 6, 5)).unwrap();
    assert!(again.crossings.is_empty());
    assert!(!again.changes(&alert));

    prices.insert(d(2024, 6, 5), PricePoint::new(dec!(200), "USD"));
    let rewritten = check_alert(&alert, &prices, &rates, d(2024, 6, 5)).unwrap();
    assert_eq!(rewritten.crossings.len(), 1);
    assert_eq!(rewritten.crossings[0].direction, CrossingDirection::Up);
}

/// A level set below a price of 110 starts Above: nothing is logged until the price falls under
/// it. A quote after `today` is not read — Jun 6 at 90 would cross, but today is Jun 5.
#[test]
fn the_trigger_starts_from_where_the_price_is() {
    let alert = SecurityAlert::price(AAPL, dec!(100), "USD", d(2024, 6, 3));
    let prices = closes(
        "USD",
        &[
            (d(2024, 6, 3), dec!(110)),
            (d(2024, 6, 5), dec!(100)),
            (d(2024, 6, 6), dec!(90)),
        ],
    );
    let check = check_alert(&alert, &prices, &FakeRates::new(), d(2024, 6, 5)).unwrap();
    assert!(
        check.crossings.is_empty(),
        "100 is at the level, which counts as above"
    );
    assert_eq!(check.side, Some(AlertSide::Above));
    assert_eq!(check.checked_through, Some(d(2024, 6, 5)));
}

/// Review on Jul 1: nothing on Jun 30, one entry from Jul 1 on, and not a second one after.
#[test]
fn a_date_rule_is_logged_once() {
    let mut alert = SecurityAlert::date_reached(AAPL, d(2024, 7, 1), d(2024, 6, 1));
    let rates = FakeRates::new();
    let none = BTreeMap::new();

    assert!(
        check_alert(&alert, &none, &rates, d(2024, 6, 30))
            .unwrap()
            .crossings
            .is_empty()
    );
    let reached = check_alert(&alert, &none, &rates, d(2024, 7, 2)).unwrap();
    assert_eq!(reached.crossings.len(), 1);
    assert_eq!(reached.crossings[0].date, d(2024, 7, 1));
    assert_eq!(reached.crossings[0].direction, CrossingDirection::Reached);

    apply(&mut alert, &reached);
    assert!(
        check_alert(&alert, &none, &rates, d(2024, 7, 3))
            .unwrap()
            .crossings
            .is_empty()
    );
}

/// Level 180 against a close of 150: 180 / 150 − 1 = 0.2, the price has to rise 20 %.
/// Level 120 against the same close: 120 / 150 − 1 = −0.2.
#[test]
fn the_status_says_how_far_the_level_is() {
    let prices = closes("USD", &[(d(2024, 6, 5), dec!(150))]);
    let rates = FakeRates::new();

    let above = SecurityAlert::price(AAPL, dec!(180), "USD", d(2024, 6, 1));
    let status = alert_status(&above, &prices, &rates, d(2024, 6, 6)).unwrap();
    assert_eq!(status.distance, Some(dec!(0.2)));
    assert_eq!(status.side, Some(AlertSide::Below));
    assert_eq!(status.price_date, Some(d(2024, 6, 5)));

    let below = SecurityAlert::price(AAPL, dec!(120), "USD", d(2024, 6, 1));
    let status = alert_status(&below, &prices, &rates, d(2024, 6, 6)).unwrap();
    assert_eq!(status.distance, Some(dec!(-0.2)));
}

/// No quote at all, or no rate for a foreign quote, is missing data — not "far from the level".
#[test]
fn a_status_without_market_data_is_an_error() {
    let rates = FakeRates::new();
    let alert = SecurityAlert::price(AAPL, dec!(180), "EUR", d(2024, 6, 1));

    assert!(matches!(
        alert_status(&alert, &BTreeMap::new(), &rates, d(2024, 6, 5)),
        Err(Error::MissingMarketData { kind: "price", .. })
    ));
    let usd = closes("USD", &[(d(2024, 6, 5), dec!(200))]);
    assert!(matches!(
        alert_status(&alert, &usd, &rates, d(2024, 6, 5)),
        Err(Error::MissingMarketData { kind: "fx rate", .. })
    ));
}

/// The simulator's close: 180 × 1.01 = 181.80 from below, 180 × 0.99 = 178.20 from above.
#[test]
fn a_simulated_close_lands_on_the_other_side() {
    let mut alert = SecurityAlert::price(AAPL, dec!(180), "USD", d(2024, 6, 1));
    assert_eq!(crossing_close(&alert), Some(dec!(181.80)));
    alert.side = Some(AlertSide::Above);
    assert_eq!(crossing_close(&alert), Some(dec!(178.20)));
}

/// A rise-above-100 rule. Jun 1: 95, below — the start. Jun 3: 105, up — logged.
/// Jun 4: 95, down — the side changes but a rising rule stays quiet. Jun 5: 105, up — logged again.
#[test]
fn a_directed_trigger_logs_only_its_own_crossings() {
    let alert =
        SecurityAlert::price(AAPL, dec!(100), "USD", d(2024, 6, 1)).with_direction(AlertDirection::Up);
    let prices = closes(
        "USD",
        &[
            (d(2024, 6, 1), dec!(95)),
            (d(2024, 6, 3), dec!(105)),
            (d(2024, 6, 4), dec!(95)),
            (d(2024, 6, 5), dec!(105)),
        ],
    );
    let check = check_alert(&alert, &prices, &FakeRates::new(), d(2024, 6, 5)).unwrap();

    let dates: Vec<_> = check.crossings.iter().map(|c| (c.date, c.direction)).collect();
    assert_eq!(
        dates,
        [
            (d(2024, 6, 3), CrossingDirection::Up),
            (d(2024, 6, 5), CrossingDirection::Up)
        ]
    );
    assert_eq!(check.side, Some(AlertSide::Above));
}
