use super::*;
use crate::error::Result;
use crate::market::{FetchPolicy, SecurityMatch, SecuritySearch};
use crate::model::SecurityKind;

struct TwoListings;

fn listing(symbol: &str, currency: &str, history: Option<bool>) -> SecurityMatch {
    SecurityMatch {
        source: "fake".into(),
        symbol: symbol.into(),
        name: "iShares MSCI Global Semiconductors".into(),
        exchange: None,
        mic: None,
        kind: SecurityKind::Etf,
        currency: Some(currency.into()),
        isin: None,
        has_history: history,
        last_close: None,
    }
}

impl SecuritySearch for TwoListings {
    fn id(&self) -> &'static str {
        "fake"
    }

    fn search(&self, _: &str) -> Result<Vec<SecurityMatch>> {
        Ok(vec![
            listing("IE000I8KRLL9.SG", "EUR", None),
            listing("SEMI.AS", "USD", None),
            listing("SEC0.DE", "EUR", None),
        ])
    }

    fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>> {
        Ok(Some(match symbol {
            "IE000I8KRLL9.SG" => listing(symbol, "EUR", Some(false)),
            "SEMI.AS" => listing(symbol, "USD", Some(true)),
            _ => listing(symbol, "EUR", Some(true)),
        }))
    }
}

fn service() -> MarketDataService {
    MarketDataService::new()
        .with_policy(FetchPolicy::none())
        .with_search(Box::new(TwoListings))
}

#[test]
fn a_listing_without_history_never_wins() {
    let found = service().resolve("IE000I8KRLL9").unwrap().unwrap();
    assert_ne!(found.symbol, "IE000I8KRLL9.SG");
    assert_eq!(found.has_history, Some(true));
    assert_eq!(found.isin.as_deref(), Some("IE000I8KRLL9"));
}

#[test]
fn the_preferred_currency_decides_between_working_listings() {
    let service = service();
    assert_eq!(
        service.resolve("IE000I8KRLL9").unwrap().unwrap().symbol,
        "SEMI.AS"
    );
    assert_eq!(
        service
            .resolve_preferring("IE000I8KRLL9", Some("EUR"))
            .unwrap()
            .unwrap()
            .symbol,
        "SEC0.DE"
    );
}
