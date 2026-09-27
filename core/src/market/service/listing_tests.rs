use super::*;
use crate::error::Result;
use crate::market::{FetchPolicy, Listing, ListingDirectory, SecurityMatch, SecuritySearch};
use crate::model::SecurityKind;
use rust_decimal_macros::dec;

struct ThreeMarkets;

impl ListingDirectory for ThreeMarkets {
    fn id(&self) -> &'static str {
        "fake-directory"
    }

    fn listings(&self, isin: &str) -> Result<Vec<Listing>> {
        Ok(["XETR", "XLON", "XTAE"]
            .iter()
            .zip(["EUNL", "IWDA", "SWDA"])
            .map(|(mic, ticker)| Listing {
                isin: isin.to_string(),
                mic: (*mic).to_string(),
                ticker: ticker.to_string(),
                exchange: crate::market::mic::market_name(mic).map(str::to_string),
                name: None,
                symbol: None,
                currency: None,
                has_history: None,
                last_close: None,
                source: "fake-directory".to_string(),
            })
            .collect())
    }
}

struct FakeQuotes;

impl SecuritySearch for FakeQuotes {
    fn id(&self) -> &'static str {
        "fake"
    }

    /// What a provider search returns for a bare US ticker: the same instrument on two
    /// venues, plus an unrelated company that merely starts with the same letters.
    fn search(&self, query: &str) -> Result<Vec<SecurityMatch>> {
        if query != "AWK" {
            return Ok(Vec::new());
        }
        Ok([
            ("AWK", Some("XNYS")),
            ("AWK.DE", Some("XFRA")),
            ("AWKR", Some("XNAS")),
        ]
        .into_iter()
        .map(|(symbol, mic)| SecurityMatch {
            source: "fake".into(),
            symbol: symbol.into(),
            name: "American Water Works".into(),
            exchange: None,
            mic: mic.map(str::to_string),
            kind: SecurityKind::Stock,
            currency: None,
            isin: None,
            has_history: None,
            last_close: None,
        })
        .collect())
    }

    fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>> {
        let (currency, history, close) = match symbol {
            "EUNL.DE" => ("EUR", true, dec!(127.455)),
            "IWDA.L" => ("USD", true, dec!(148.09)),
            _ => ("ILS", false, dec!(0)),
        };
        Ok(Some(SecurityMatch {
            source: "fake".into(),
            symbol: symbol.into(),
            name: "iShares Core MSCI World".into(),
            exchange: None,
            mic: None,
            kind: SecurityKind::Etf,
            currency: Some(currency.into()),
            isin: None,
            has_history: Some(history),
            last_close: Some(close),
        }))
    }

    fn symbol_for(&self, ticker: &str, mic: &str) -> Option<String> {
        let suffix = match mic {
            "XETR" => ".DE",
            "XLON" => ".L",
            "XTAE" => ".TA",
            _ => return None,
        };
        Some(format!("{ticker}{suffix}"))
    }
}

fn service() -> MarketDataService {
    MarketDataService::new()
        .with_policy(FetchPolicy::none())
        .with_search(Box::new(FakeQuotes))
        .with_directory(Box::new(ThreeMarkets))
}

#[test]
fn an_instrument_without_an_isin_gets_its_venues_from_the_search() {
    let found = service().listings_by_symbol("AWK").unwrap();
    assert_eq!(
        found.iter().map(|l| l.mic.as_str()).collect::<Vec<_>>(),
        ["XNYS", "XFRA"],
        "AWKR is another company, not another venue of AWK"
    );
    assert!(
        found.iter().all(|l| l.isin.is_empty()),
        "there is no ISIN to record"
    );
    assert_eq!(found[0].symbol.as_deref(), Some("AWK"));
    assert_eq!(found[1].symbol.as_deref(), Some("AWK.DE"));
}

#[test]
fn a_venue_the_source_cannot_name_is_not_a_listing() {
    // The suffixed symbol is what the search is run for, and only the base ticker matches.
    assert!(service().listings_by_symbol("EUNL.DE").unwrap().is_empty());
}

#[test]
fn listings_are_named_by_the_quote_provider() {
    let found = service().listings("IE00B4L5Y983").unwrap();
    let symbols: Vec<&str> = found.iter().filter_map(|l| l.symbol.as_deref()).collect();
    assert_eq!(symbols, ["EUNL.DE", "IWDA.L", "SWDA.TA"]);
    assert!(
        found.iter().all(|l| l.currency.is_none()),
        "the currency was not asked for"
    );
}

#[test]
fn the_base_currency_decides_which_listing_wins() {
    let best = service()
        .best_listing("IE00B4L5Y983", Some("EUR"))
        .unwrap()
        .unwrap();
    assert_eq!(best.symbol.as_deref(), Some("EUNL.DE"));
    assert_eq!(best.currency.as_deref(), Some("EUR"));
    assert_eq!(best.last_close, Some(dec!(127.455)));
}

#[test]
fn a_working_listing_beats_the_right_currency() {
    let best = service()
        .best_listing("IE00B4L5Y983", Some("CHF"))
        .unwrap()
        .unwrap();
    assert_eq!(best.symbol.as_deref(), Some("EUNL.DE"));
}

#[test]
fn a_listing_without_prices_is_not_usable() {
    let probed = service().probe_listings(service().listings("IE00B4L5Y983").unwrap(), 10);
    let tel_aviv = probed.iter().find(|l| l.mic == "XTAE").unwrap();
    assert_eq!(tel_aviv.has_history, Some(false));
    assert!(!tel_aviv.is_usable());
}
