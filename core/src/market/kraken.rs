use super::{DateRange, Quote, QuoteProvider};
use crate::error::{Error, Result};
use crate::model::{Security, SecurityKind};
use crate::money::normalize_currency;
use chrono::DateTime;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// Kraken's public daily candles: crypto only, no key. The endpoint returns at most the last 720
/// candles whatever `since` says, so older history is simply absent, never an error.
pub struct KrakenProvider {
    http_timeout: Duration,
}

impl Default for KrakenProvider {
    fn default() -> Self {
        KrakenProvider {
            http_timeout: Duration::from_secs(20),
        }
    }
}

impl KrakenProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "kraken";

    pub fn new() -> Self {
        Self::default()
    }

    /// The pair's quote currency is its last three letters: `XBTEUR`, `XXBTZEUR` and `ETHUSD` alike.
    fn quote_currency(pair: &str) -> Option<String> {
        let pair = pair.trim();
        (pair.len() > 3 && pair.is_ascii()).then(|| normalize_currency(&pair[pair.len() - 3..]))
    }

    pub(crate) fn parse(
        security: &Security,
        currency: &str,
        body: &str,
        range: DateRange,
    ) -> Result<Vec<Quote>> {
        let bad = |detail: String| Error::BadProviderData {
            provider: KrakenProvider::ID,
            detail,
        };
        let json: serde_json::Value =
            serde_json::from_str(body).map_err(|e| bad(format!("malformed JSON: {e}")))?;
        if let Some(first) = json["error"].as_array().and_then(|e| e.first()) {
            return Err(bad(first.to_string()));
        }
        // `result` holds the candles under Kraken's own spelling of the pair, beside `last`.
        let candles = json["result"]
            .as_object()
            .and_then(|r| r.iter().find(|(k, _)| *k != "last"))
            .and_then(|(_, v)| v.as_array())
            .ok_or_else(|| bad("no candles".into()))?;
        let mut quotes = Vec::new();
        for candle in candles {
            let (Some(time), Some(close)) = (candle[0].as_i64(), candle[4].as_str()) else {
                continue;
            };
            let Some(date) = DateTime::from_timestamp(time, 0).map(|t| t.date_naive()) else {
                continue;
            };
            let Ok(close) = Decimal::from_str(close) else {
                continue;
            };
            if date >= range.from && date <= range.to && close > Decimal::ZERO {
                quotes.push(Quote {
                    security_id: security.id.clone(),
                    date,
                    close,
                    currency: currency.to_string(),
                    source: KrakenProvider::ID.to_string(),
                });
            }
        }
        Ok(quotes)
    }
}

impl QuoteProvider for KrakenProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn covers(&self, security: &Security) -> bool {
        security.kind == SecurityKind::Crypto
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        let pair = security.provider_symbol();
        let currency =
            Self::quote_currency(pair).ok_or_else(|| Error::Invalid(format!("kraken pair {pair}")))?;
        let since = range
            .from
            .and_hms_opt(0, 0, 0)
            .map(|t| t.and_utc().timestamp())
            .unwrap_or(0);
        let url = format!("https://api.kraken.com/0/public/OHLC?pair={pair}&interval=1440&since={since}");
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let body = agent.get(url).call()?.body_mut().read_to_string()?;
        Self::parse(security, &currency, &body, range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn btc() -> Security {
        Security::new("BTC", "Bitcoin", "EUR", SecurityKind::Crypto).with_source(KrakenProvider::ID, "XBTEUR")
    }

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn reads_the_close_of_each_candle_inside_the_range() {
        // 1727568000 = 2024-09-29, 1727654400 = 2024-09-30; only the second is in range.
        let body = r#"{"error":[],"result":{"XXBTZEUR":[
            [1727568000,"58981.1","59095.4","58636.4","58746.8","58843.2","160.1",10825],
            [1727654400,"58735.1","58750.0","56450.1","56874.6","57215.5","629.1",26317]],"last":1727654400}}"#;
        let range = DateRange::new(d("2024-09-30"), d("2024-10-01"));
        let quotes = KrakenProvider::parse(&btc(), "EUR", body, range).unwrap();
        assert_eq!(quotes.len(), 1);
        assert_eq!(
            (quotes[0].date, quotes[0].close),
            (d("2024-09-30"), Decimal::from_str("56874.6").unwrap())
        );
    }

    #[test]
    fn an_error_array_is_the_source_s_refusal() {
        let body = r#"{"error":["EQuery:Unknown asset pair"]}"#;
        let range = DateRange::new(d("2024-09-30"), d("2024-10-01"));
        assert!(KrakenProvider::parse(&btc(), "EUR", body, range).is_err());
    }

    #[test]
    fn the_currency_is_the_pair_s_last_three_letters() {
        assert_eq!(KrakenProvider::quote_currency("XXBTZEUR").as_deref(), Some("EUR"));
        assert_eq!(KrakenProvider::quote_currency("ETHUSD").as_deref(), Some("USD"));
    }

    #[test]
    fn covers_crypto_only() {
        let stock = Security::new("AAPL", "Apple", "USD", SecurityKind::Stock);
        assert!(KrakenProvider::new().covers(&btc()) && !KrakenProvider::new().covers(&stock));
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_recent_bitcoin_closes() {
        let today = chrono::Utc::now().date_naive();
        let range = DateRange::new(today - chrono::Duration::days(10), today);
        assert!(!KrakenProvider::new().fetch(&btc(), range).unwrap().is_empty());
    }
}
