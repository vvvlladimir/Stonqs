use super::*;
use sq_core::calc::{RealPerformance, RealReturn};

fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn sample() -> RealPerformance {
    RealPerformance {
        twr: RealReturn {
            region: "DE".into(),
            from: day(2023, 7, 1),
            // Earlier than the period asked for: July's index is not published yet.
            to: day(2024, 6, 30),
            factor: dec!(1.04),
            inflation: dec!(0.04),
            nominal: dec!(0.09),
            real: dec!(0.048077),
            real_annualized: Some(dec!(0.048077)),
        },
        xirr: Some(dec!(0.0312)),
    }
}

/// Every return and the inflation factor cross as strings; the region is a code, never a name.
#[test]
fn a_real_return_crosses_as_strings_and_a_region_code() {
    let json = serde_json::to_value(sample()).unwrap();
    assert_eq!(
        keys(&json["twr"]),
        vec![
            "factor",
            "from",
            "inflation",
            "nominal",
            "real",
            "real_annualized",
            "region",
            "to"
        ]
    );
    assert_eq!(json["twr"]["region"], Value::String("DE".into()));
    assert_eq!(json["twr"]["factor"], Value::String("1.04".into()));
    assert_eq!(json["twr"]["inflation"], Value::String("0.04".into()));
    assert_eq!(json["twr"]["real"], Value::String("0.048077".into()));
    assert_eq!(json["xirr"], Value::String("0.0312".into()));
}

/// The window is carried, not inferred: the frontend says which months were deflated.
#[test]
fn the_deflated_window_is_reported_rather_than_the_one_asked_for() {
    let json = serde_json::to_value(sample()).unwrap();
    assert_eq!(json["twr"]["from"], Value::String("2023-07-01".into()));
    assert_eq!(json["twr"]["to"], Value::String("2024-06-30".into()));
}

/// A portfolio with no region gets no object at all, which is not the same as a zero return.
#[test]
fn an_unset_region_is_null_rather_than_a_zero() {
    let json = serde_json::to_value(Option::<RealPerformance>::None).unwrap();
    assert_eq!(json, Value::Null);

    let mut no_irr = sample();
    no_irr.xirr = None;
    assert_eq!(serde_json::to_value(no_irr).unwrap()["xirr"], Value::Null);
}

/// Region codes are the picker's whole vocabulary — no names, so the frontend can translate.
#[test]
fn the_region_list_is_codes_covering_both_sources() {
    let regions = sq_core::inflation::regions();
    assert!(regions.contains(&"EA") && regions.contains(&"DE"));
    // Beyond Europe, which only the worldwide source answers.
    assert!(regions.contains(&"JP") && regions.contains(&"RU") && regions.contains(&"BR"));
    assert!(
        regions
            .iter()
            .all(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()))
    );
    assert!(
        regions.windows(2).all(|w| w[0] < w[1]),
        "sorted and free of duplicates"
    );
}
