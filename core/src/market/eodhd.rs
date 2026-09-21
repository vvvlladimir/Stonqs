use super::{DateRange, Quote, QuoteProvider};
use crate::error::{Error, Result};
use crate::model::Security;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// EODHD's end-of-day prices for most of the world's exchanges (`TICKER.EXCHANGE`), behind a key.
pub struct EodhdProvider {
    key: String,
    http_timeout: Duration,
}

impl EodhdProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "eodhd";

    pub fn new(key: impl Into<String>) -> Self {
        EodhdProvider {
            key: key.into(),
            http_timeout: Duration::from_secs(20),
        }
    }

    /// `close` is split-adjusted like every stored quote; `adjusted_close` also folds dividends in
    /// and is not used. The endpoint names no currency, so the instrument's own is assumed and the
    /// fallback guard's ratio check is what catches a venue quoting in pence.
    pub(crate) fn parse(security: &Security, body: &str) -> Result<Vec<Quote>> {
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(body).map_err(|e| Error::BadProviderData {
                provider: Self::ID,
                detail: format!("malformed JSON: {e}"),
            })?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let date = NaiveDate::parse_from_str(row["date"].as_str()?, "%Y-%m-%d").ok()?;
                // serde_json prints an f64 back in its shortest form, which is the decimal sent.
                let close = Decimal::from_str(&row["close"].as_f64()?.to_string()).ok()?;
                (close > Decimal::ZERO).then(|| Quote {
                    security_id: security.id.clone(),
                    date,
                    close,
                    currency: security.currency.clone(),
                    source: Self::ID.to_string(),
                })
            })
            .collect())
    }
}

impl QuoteProvider for EodhdProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        let url = format!(
            "https://eodhd.com/api/eod/{}?from={}&to={}&period=d&fmt=json&api_token={}",
            security.provider_symbol(),
            range.from,
            range.to,
            self.key
        );
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        Self::parse(security, &agent.get(url).call()?.body_mut().read_to_string()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    fn aapl() -> Security {
        Security::new("AAPL", "Apple", "USD", SecurityKind::Stock).with_source(EodhdProvider::ID, "AAPL.US")
    }

    #[test]
    fn reads_the_split_adjusted_close_not_the_dividend_adjusted_one() {
        let body = r#"[{"date":"2024-06-03","close":194.03,"adjusted_close":192.1979}]"#;
        let quotes = EodhdProvider::parse(&aapl(), body).unwrap();
        assert_eq!(
            (quotes[0].close, quotes[0].currency.as_str()),
            (dec!(194.03), "USD")
        );
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_aapl_with_the_demo_key() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let quotes = EodhdProvider::new("demo")
            .fetch(&aapl(), DateRange::new(d("2024-06-03"), d("2024-06-07")))
            .unwrap();
        assert_eq!(quotes.len(), 5);
    }
}
