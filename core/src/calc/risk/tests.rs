use super::*;
use crate::money::normalize_currency;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// A no-flow series with business-day dates.
fn series(values: &[(NaiveDate, Decimal)]) -> ValueSeries {
    ValueSeries {
        base_currency: normalize_currency("EUR"),
        dates: values.iter().map(|(d, _)| *d).collect(),
        total_value_base: values.iter().map(|(_, v)| *v).collect(),
        external_flow_base: values.iter().map(|_| Decimal::ZERO).collect(),
    }
}

/// Hand-built drawdown: 100 -> 120 -> 90 -> 110 -> 130 gives
/// peak 1.2, trough 0.9, depth `0.9 / 1.2 - 1 = -0.25`, recovery Friday.
#[test]
fn max_drawdown_on_a_hand_made_series() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(120)),
        (d(2024, 6, 5), dec!(90)),
        (d(2024, 6, 6), dec!(110)),
        (d(2024, 6, 7), dec!(130)),
    ]);
    let dd = risk_metrics(&s, 0.0).max_drawdown.unwrap();
    assert_eq!(dd.peak, d(2024, 6, 4));
    assert_eq!(dd.trough, d(2024, 6, 5));
    assert_eq!(dd.recovered, Some(d(2024, 6, 7)));
    assert!((dd.depth - (-0.25)).abs() < 1e-12, "depth = {}", dd.depth);
}

/// Longhand check: returns +0.1, -0.1, +0.1; mean 0.0333333,
/// sample variance 0.01333333, daily sigma 0.11547005, annualized 1.8330303.
#[test]
fn volatility_matches_longhand_arithmetic() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(110)),
        (d(2024, 6, 5), dec!(99)),
        (d(2024, 6, 6), dec!(108.9)),
    ]);
    let m = risk_metrics(&s, 0.0);
    assert_eq!(m.days, 3);
    assert!((m.volatility - 1.8330303).abs() < 1e-6, "vol = {}", m.volatility);
    assert!((m.positive_days_share - 2.0 / 3.0).abs() < 1e-12);
    // Two best days tie at +0.1; date selection is an implementation detail.
    assert!((m.best_day.unwrap().1 - 0.1).abs() < 1e-12);
    assert_eq!(m.worst_day.unwrap().0, d(2024, 6, 5));
}

/// Weekend prices do not add zero returns or dilute volatility.
#[test]
fn weekend_days_do_not_dilute_volatility() {
    let with_weekend = series(&[
        (d(2024, 6, 7), dec!(100)),  // Friday
        (d(2024, 6, 8), dec!(100)),  // Saturday
        (d(2024, 6, 9), dec!(100)),  // Sunday
        (d(2024, 6, 10), dec!(110)), // Monday
        (d(2024, 6, 11), dec!(99)),
        (d(2024, 6, 12), dec!(108.9)),
    ]);
    assert_eq!(risk_metrics(&with_weekend, 0.0).days, 3);
}

/// Lists all episodes: 100 -> 90 -> 100 -> 80 -> 100 produces
/// depths -0.1 and -0.2; the deeper episode sorts first.
#[test]
fn drawdowns_lists_every_episode_deepest_first() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(90)),
        (d(2024, 6, 5), dec!(100)),
        (d(2024, 6, 6), dec!(80)),
        (d(2024, 6, 7), dec!(100)),
    ]);
    let episodes = drawdowns(&return_series(&s));
    assert_eq!(episodes.len(), 2);
    assert!((episodes[0].depth - (-0.2)).abs() < 1e-12, "{:?}", episodes[0]);
    assert_eq!(episodes[0].trough, d(2024, 6, 6));
    assert_eq!(episodes[0].recovered, Some(d(2024, 6, 7)));
    assert!((episodes[1].depth - (-0.1)).abs() < 1e-12);
    assert_eq!(episodes[1].recovered, Some(d(2024, 6, 5)));
    // The metric takes the first item after depth sorting.
    assert_eq!(risk_metrics(&s, 0.0).max_drawdown.unwrap(), episodes[0]);
}

/// An unrecovered episode remains with `recovered = None`.
#[test]
fn an_unrecovered_drawdown_is_still_reported() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(120)),
        (d(2024, 6, 5), dec!(90)),
    ]);
    let episodes = drawdowns(&return_series(&s));
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].peak, d(2024, 6, 4));
    assert_eq!(episodes[0].recovered, None);
}

/// Drawdown chart returns to zero after recovery.
#[test]
fn drawdown_series_is_zero_at_every_peak() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(90)),
        (d(2024, 6, 5), dec!(100)),
    ]);
    let under = drawdown_series(&return_series(&s));
    // The first return is Tuesday; Monday has no preceding value.
    assert_eq!(under.dates, vec![d(2024, 6, 4), d(2024, 6, 5)]);
    assert!((under.values[0] - (-0.1)).abs() < 1e-12);
    assert!(under.values[1].abs() < 1e-12);
}

/// Window-3 check: returns alternate +0.1/-0.1; both windows have
/// variance 0.01333333 and annualized volatility 1.8330303.
#[test]
fn rolling_volatility_matches_longhand_arithmetic() {
    let s = series(&[
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(110)),
        (d(2024, 6, 5), dec!(99)),
        (d(2024, 6, 6), dec!(108.9)),
        (d(2024, 6, 7), dec!(98.01)),
    ]);
    let vol = rolling_volatility(&return_series(&s), 3);
    assert_eq!(vol.dates, vec![d(2024, 6, 6), d(2024, 6, 7)]);
    assert!((vol.values[0] - 1.8330303).abs() < 1e-6, "{}", vol.values[0]);
    assert!((vol.values[1] - 1.8330303).abs() < 1e-6, "{}", vol.values[1]);
}

/// A window wider than the series yields no points, not zeros.
#[test]
fn rolling_volatility_needs_a_full_window() {
    let s = series(&[(d(2024, 6, 3), dec!(100)), (d(2024, 6, 4), dec!(110))]);
    assert!(rolling_volatility(&return_series(&s), 63).is_empty());
}

/// Deposits must not appear as returns or drawdowns.
#[test]
fn deposits_do_not_create_returns() {
    let s = ValueSeries {
        base_currency: normalize_currency("EUR"),
        dates: vec![d(2024, 6, 3), d(2024, 6, 4)],
        total_value_base: vec![dec!(1000), dec!(2000)],
        external_flow_base: vec![Decimal::ZERO, dec!(1000)],
    };
    let m = risk_metrics(&s, 0.0);
    assert_eq!(m.days, 1);
    assert!(m.volatility.abs() < 1e-12);
    assert!(m.max_drawdown.is_none());
}
