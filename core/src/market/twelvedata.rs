use super::{DateRange, Quote, QuoteProvider};
use crate::error::{Error, Result};
use crate::fx::{FxProvider, FxRate};
use crate::model::Security;
use crate::money::{major_currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// Twelve Data's `time_series`: stocks, ETFs, FX and crypto on one endpoint, behind a key.
pub struct TwelveDataProvider {
    key: String,
    http_timeout: Duration,
}

/// What one `time_series` answer held, before it becomes quotes or rates.
struct Series {
    currency: Option<String>,
    closes: Vec<(NaiveDate, Decimal)>,
}

impl TwelveDataProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "twelvedata";

    pub fn new(key: impl Into<String>) -> Self {
        TwelveDataProvider {
            key: key.into(),
            http_timeout: Duration::from_secs(20),
        }
    }

    /// Twelve Data answers errors with HTTP 200 and a `code` in the body; this reads it as the
    /// status it stands for, so a rejected key or a throttle is told apart like anywhere else.
    fn parse(body: &str) -> Result<Series> {
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: Self::ID,
            detail: format!("malformed JSON: {e}"),
        })?;
        if json["status"] == "error" {
            let message = json["message"].as_str().unwrap_or("error").to_string();
            return Err(match json["code"].as_u64() {
                Some(401 | 403) => Error::Unauthorized(message),
                Some(429) => Error::RateLimited(message),
                Some(500..=599) => Error::Unavailable(message),
                _ => Error::BadProviderData {
                    provider: Self::ID,
                    detail: message,
                },
            });
        }
        let values = json["values"].as_array().ok_or_else(|| Error::BadProviderData {
            provider: Self::ID,
            detail: "no values".into(),
        })?;
        let mut closes: Vec<(NaiveDate, Decimal)> = values
            .iter()
            .filter_map(|v| {
                let date = NaiveDate::parse_from_str(v["datetime"].as_str()?.get(..10)?, "%Y-%m-%d").ok()?;
                let close = Decimal::from_str(v["close"].as_str()?).ok()?;
                (close > Decimal::ZERO).then_some((date, close))
            })
            .collect();
        closes.sort_by_key(|(d, _)| *d);
        Ok(Series {
            currency: json["meta"]["currency"].as_str().map(str::to_string),
            closes,
        })
    }

    fn series(&self, symbol: &str, mic: Option<&str>, range: DateRange) -> Result<Series> {
        let mut url = format!(
            "https://api.twelvedata.com/time_series?symbol={}&interval=1day&outputsize=5000&start_date={}&end_date={}&apikey={}",
            symbol.replace('/', "%2F"),
            range.from,
            // `end_date` is exclusive.
            range.to.succ_opt().unwrap_or(range.to),
            self.key
        );
        if let Some(mic) = mic {
            url.push_str(&format!("&mic_code={mic}"));
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        Self::parse(&agent.get(url).call()?.body_mut().read_to_string()?)
    }
}

impl QuoteProvider for TwelveDataProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        let series = self.series(security.provider_symbol(), security.mic.as_deref(), range)?;
        let (currency, factor) = match series.currency.as_deref() {
            Some(raw) => major_currency(raw),
            None => (security.currency.clone(), Decimal::ONE),
        };
        Ok(series
            .closes
            .into_iter()
            .filter(|(d, _)| *d >= range.from && *d <= range.to)
            .map(|(date, close)| Quote {
                security_id: security.id.clone(),
                date,
                close: close * factor,
                currency: currency.clone(),
                source: Self::ID.to_string(),
            })
            .collect())
    }
}

impl FxProvider for TwelveDataProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        if base == quote {
            return Ok(Vec::new());
        }
        Ok(self
            .series(&format!("{base}/{quote}"), None, range)?
            .closes
            .into_iter()
            .filter(|(d, _)| *d >= range.from && *d <= range.to)
            .map(|(date, rate)| FxRate::new(&base, &quote, date, rate))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    #[test]
    fn reads_closes_oldest_first_with_the_currency_of_the_meta() {
        let body = r#"{"meta":{"symbol":"AAPL","currency":"USD"},"values":[
            {"datetime":"2024-06-07","close":"196.89000"},{"datetime":"2024-06-06","close":"194.48000"}],"status":"ok"}"#;
        let series = TwelveDataProvider::parse(body).unwrap();
        assert_eq!(series.currency.as_deref(), Some("USD"));
        assert_eq!(series.closes[0].1, dec!(194.48));
    }

    #[test]
    fn an_error_in_the_body_is_the_status_it_names() {
        let refused = r#"{"code":401,"message":"**apikey** parameter is incorrect","status":"error"}"#;
        assert!(matches!(
            TwelveDataProvider::parse(refused),
            Err(Error::Unauthorized(_))
        ));
        let throttled = r#"{"code":429,"message":"run out of API credits","status":"error"}"#;
        assert!(matches!(
            TwelveDataProvider::parse(throttled),
            Err(Error::RateLimited(_))
        ));
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_aapl_with_the_demo_key() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let aapl = Security::new("AAPL", "Apple", "USD", SecurityKind::Stock)
            .with_source(TwelveDataProvider::ID, "AAPL");
        let quotes = QuoteProvider::fetch(
            &TwelveDataProvider::new("demo"),
            &aapl,
            DateRange::new(d("2024-06-03"), d("2024-06-07")),
        )
        .unwrap();
        assert_eq!(quotes.len(), 5);
    }
}
