use super::*;
use crate::model::SecurityKind;
use chrono::NaiveDate;
use rust_decimal_macros::dec;

fn aapl() -> Security {
    Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("yahoo", "AAPL")
}

const SAMPLE: &str = r#"{
        "chart": {
            "result": [{
                "meta": {"currency": "USD", "symbol": "AAPL", "gmtoffset": -14400},
                "timestamp": [1717421400, 1717507800, 1717594200],
                "indicators": {"quote": [{"close": [194.02999877929688, null, 195.8699951171875]}]}
            }],
            "error": null
        }
    }"#;

#[test]
fn every_supported_market_can_be_named() {
    for m in crate::market::mic::supported_mics() {
        assert!(
            YAHOO_SUFFIXES.iter().any(|(mic, _)| *mic == m),
            "venue {m} has no Yahoo suffix"
        );
    }
}

#[test]
fn the_suffix_names_the_venue_and_beats_the_exchange_name() {
    assert_eq!(YahooProvider::mic_of("EUNL.DE", Some("NYSE")), Some("XETR"));
    assert_eq!(YahooProvider::mic_of("iwda.l", None), Some("XLON"));
    assert_eq!(
        YahooProvider::mic_of("SAP.XX", Some("NYSE")),
        None,
        "an unknown suffix is an unsupported venue, not a US listing"
    );
}

#[test]
fn a_suffixless_symbol_takes_the_venue_from_the_exchange_name() {
    assert_eq!(YahooProvider::mic_of("AWK", Some("NYSE")), Some("XNYS"));
    assert_eq!(YahooProvider::mic_of("AAPL", Some("NasdaqGS")), Some("XNAS"));
    assert_eq!(YahooProvider::mic_of("SPY", Some("NYSEArca")), Some("ARCX"));
    assert_eq!(YahooProvider::mic_of("AWK", None), None);
    assert_eq!(YahooProvider::mic_of("AWK", Some("Bombay")), None);
}

#[test]
fn every_venue_named_by_mic_of_is_a_supported_market() {
    for (_, mic) in YAHOO_US_EXCHANGES {
        assert!(
            crate::market::mic::market_name(mic).is_some(),
            "venue {mic} is not a supported market"
        );
    }
}

#[test]
fn a_listing_is_named_by_ticker_and_exchange() {
    let yahoo = YahooProvider::new();
    assert_eq!(yahoo.symbol_for("EUNL", "XETR").as_deref(), Some("EUNL.DE"));
    assert_eq!(yahoo.symbol_for("AAPL", "XNAS").as_deref(), Some("AAPL"));
    assert_eq!(yahoo.symbol_for("EUNL", "XXXX"), None);
}

#[test]
fn london_pence_are_converted_to_pounds() {
    let swda = Security::new("SWDA.L", "iShares Core MSCI World", "GBP", SecurityKind::Etf)
        .with_source("yahoo", "SWDA.L");
    let body = r#"{
            "chart": {
                "result": [{
                    "meta": {"currency": "GBp", "symbol": "SWDA.L", "gmtoffset": 3600},
                    "timestamp": [1717421400],
                    "indicators": {"quote": [{"close": [10953.0]}]}
                }],
                "error": null
            }
        }"#;
    let quotes = YahooProvider::parse_chart(&swda, body).unwrap();
    assert_eq!(quotes[0].currency, "GBP");
    assert_eq!(quotes[0].close, dec!(109.53));
}

#[test]
fn parses_chart_and_skips_null_days() {
    let quotes = YahooProvider::parse_chart(&aapl(), SAMPLE).unwrap();
    assert_eq!(quotes.len(), 2, "a day with a null price is skipped");
    assert_eq!(quotes[0].date, NaiveDate::from_ymd_opt(2024, 6, 3).unwrap());
    assert_eq!(quotes[1].date, NaiveDate::from_ymd_opt(2024, 6, 5).unwrap());
    assert_eq!(quotes[0].currency, "USD");
    assert_eq!(quotes[0].source, "yahoo");
}

/// NVIDIA's 10-for-1 on 2024-06-10 and the 0.01 USD dividend the next day, as Yahoo prints them.
#[test]
fn dividends_and_splits_come_with_the_same_response() {
    let nvda = Security::new("NVDA", "NVIDIA", "USD", SecurityKind::Stock).with_source("yahoo", "NVDA");
    let body = r#"{
            "chart": {"result": [{
                "meta": {"currency": "USD", "symbol": "NVDA", "gmtoffset": -14400},
                "timestamp": [1718026200, 1718112600],
                "indicators": {"quote": [{"close": [121.79, 120.91]}]},
                "events": {
                    "dividends": {"1718112600": {"amount": 0.01, "date": 1718112600}},
                    "splits": {"1718026200": {"date": 1718026200, "numerator": 10.0,
                                              "denominator": 1.0, "splitRatio": "10:1"}}
                }
            }], "error": null}
        }"#;
    let history = YahooProvider::parse_history(&nvda, body).unwrap();
    assert_eq!(history.quotes.len(), 2);
    assert_eq!(history.events.len(), 2);

    let split = &history.events[0];
    assert_eq!(split.date, NaiveDate::from_ymd_opt(2024, 6, 10).unwrap());
    assert_eq!(
        (split.ratio_from, split.ratio_to),
        (Some(dec!(1)), Some(dec!(10)))
    );

    let dividend = &history.events[1];
    assert_eq!(dividend.date, NaiveDate::from_ymd_opt(2024, 6, 11).unwrap());
    assert_eq!(dividend.amount, Some(dec!(0.01)));
    assert_eq!(dividend.currency.as_deref(), Some("USD"));
    assert_eq!(dividend.source.as_deref(), Some("yahoo"));
}

#[test]
fn a_response_without_events_has_none() {
    assert!(
        YahooProvider::parse_history(&aapl(), SAMPLE)
            .unwrap()
            .events
            .is_empty()
    );
}

#[test]
fn f32_noise_is_rounded_away() {
    let quotes = YahooProvider::parse_chart(&aapl(), SAMPLE).unwrap();
    assert_eq!(quotes[0].close, dec!(194.03));
    assert_eq!(quotes[1].close, dec!(195.87));
}

#[test]
fn small_prices_keep_significant_digits() {
    assert_eq!(
        YahooProvider::price_from_json(0.000001234567).unwrap(),
        dec!(0.000001234567)
    );
}

#[test]
fn reports_provider_error() {
    let body = r#"{"chart":{"result":[],"error":{"code":"Not Found","description":"No data found"}}}"#;
    assert!(matches!(
        YahooProvider::parse_chart(&aapl(), body),
        Err(Error::BadProviderData {
            provider: "yahoo",
            ..
        })
    ));
}

const SEARCH_SAMPLE: &str = r#"{
        "quotes": [
            {"exchange": "MIL", "shortname": "ISHARES CORE S&P 500 UCITS ETF ",
             "quoteType": "ETF", "symbol": "CSSPX.MI", "typeDisp": "ETF",
             "longname": "iShares Core S&P 500 UCITS ETF USD (Acc)",
             "exchDisp": "Milan", "isYahooFinance": true},
            {"index": "industry", "name": "Asset Management", "isYahooFinance": false}
        ]
    }"#;

#[test]
fn search_keeps_only_instruments() {
    let found = YahooProvider::parse_search(SEARCH_SAMPLE).unwrap();
    assert_eq!(found.len(), 1, "an industry entry is not an instrument");
    assert_eq!(found[0].symbol, "CSSPX.MI");
    assert_eq!(found[0].name, "iShares Core S&P 500 UCITS ETF USD (Acc)");
    assert_eq!(found[0].exchange.as_deref(), Some("Milan"));
    assert_eq!(found[0].kind, SecurityKind::Etf);
    assert!(!found[0].is_complete());
}

const PROFILE_SAMPLE: &str = r#"{
        "chart": {"result": [{
            "meta": {"currency": "EUR", "symbol": "CSSPX.MI", "gmtoffset": 7200,
                     "fullExchangeName": "Milan", "instrumentType": "ETF",
                     "longName": "iShares Core S&P 500 UCITS ETF USD (Acc)"},
            "timestamp": [1717421400], "indicators": {"quote": [{"close": [715.13]}]}
        }], "error": null}
    }"#;

#[test]
fn profile_fills_the_currency() {
    let found = YahooProvider::parse_profile("CSSPX.MI", PROFILE_SAMPLE)
        .unwrap()
        .unwrap();
    assert_eq!(found.currency.as_deref(), Some("EUR"));
    assert_eq!(found.name, "iShares Core S&P 500 UCITS ETF USD (Acc)");
    assert_eq!(found.has_history, Some(true));
    assert!(found.is_complete());
}

#[test]
fn a_listing_without_candles_is_not_complete() {
    let body = r#"{
            "chart": {"result": [{
                "meta": {"currency": "EUR", "symbol": "IE000I8KRLL9.SG",
                         "fullExchangeName": "Stuttgart", "instrumentType": "MUTUALFUND",
                         "shortName": "iShares MSCI Global Semiconduct", "gmtoffset": 7200,
                         "regularMarketPrice": 17.006},
                "timestamp": [], "indicators": {"quote": [{"close": []}]}
            }], "error": null}
        }"#;
    let found = YahooProvider::parse_profile("IE000I8KRLL9.SG", body)
        .unwrap()
        .unwrap();
    assert_eq!(found.currency.as_deref(), Some("EUR"), "the currency is there");
    assert_eq!(found.has_history, Some(false), "and the prices are not");
    assert!(!found.is_complete());
}

#[test]
fn query_is_escaped() {
    assert!(YahooProvider::search_url("S&P 500").contains("S%26P%20500"));
}

#[test]
#[ignore = "requires network"]
fn resolves_a_real_isin() {
    let provider = YahooProvider::new();
    let found = provider.search("IE00B5BMR087").unwrap();
    assert_eq!(found[0].symbol, "CSSPX.MI");
    let profile = provider.profile(&found[0].symbol).unwrap().unwrap();
    assert_eq!(profile.currency.as_deref(), Some("EUR"));
}
