use super::{DateRange, Quote, QuoteProvider};
use crate::error::{Error, Result};
use crate::model::Security;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;
/// Stooq CSV adapter retained as a second quote-source implementation.
pub struct StooqProvider {
    http_timeout: Duration,
}

impl Default for StooqProvider {
    fn default() -> Self {
        StooqProvider {
            http_timeout: Duration::from_secs(15),
        }
    }
}

impl StooqProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "stooq";

    pub fn new() -> Self {
        Self::default()
    }

    fn url(symbol: &str, range: DateRange) -> String {
        format!(
            "https://stooq.com/q/d/l/?s={}&d1={}&d2={}&i=d",
            symbol.to_ascii_lowercase(),
            range.from.format("%Y%m%d"),
            range.to.format("%Y%m%d"),
        )
    }
    /// Parse the flat daily CSV without requiring a general CSV dependency.
    pub(crate) fn parse_csv(security: &Security, body: &str) -> Result<Vec<Quote>> {
        let mut out = Vec::new();
        let mut lines = body.lines();

        let header = lines.next().unwrap_or_default();
        if !header.to_ascii_lowercase().starts_with("date,") {
            return Err(Error::BadProviderData {
                provider: StooqProvider::ID,
                detail: format!("unexpected first line: {header:?}"),
            });
        }

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split(',').collect();
            // Missing or non-positive closes are not valid portfolio prices.
            if cols.len() < 5 {
                continue;
            }
            let date =
                NaiveDate::parse_from_str(cols[0], "%Y-%m-%d").map_err(|e| Error::BadProviderData {
                    provider: StooqProvider::ID,
                    detail: format!("bad date {:?}: {e}", cols[0]),
                })?;
            let Ok(close) = Decimal::from_str(cols[4]) else {
                continue;
            };
            if close.is_sign_negative() || close.is_zero() {
                continue;
            }
            out.push(Quote {
                security_id: security.id.clone(),
                date,
                close,
                currency: security.currency.clone(),
                source: StooqProvider::ID.to_string(),
            });
        }
        Ok(out)
    }
}

impl QuoteProvider for StooqProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        let url = Self::url(security.provider_symbol(), range);
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let body = agent.get(&url).call()?.body_mut().read_to_string()?;
        Self::parse_csv(security, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    fn aapl() -> Security {
        Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("stooq", "aapl.us")
    }

    #[test]
    fn parses_normal_csv() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2024-06-03,192.90,194.99,192.52,194.03,50080539\n\
                   2024-06-04,194.64,195.32,193.03,194.35,47471445\n";
        let quotes = StooqProvider::parse_csv(&aapl(), csv).unwrap();
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[1].close, dec!(194.35));
        assert_eq!(quotes[0].currency, "USD");
        assert_eq!(quotes[0].source, "stooq");
    }

    #[test]
    fn rejects_error_page() {
        let err = StooqProvider::parse_csv(&aapl(), "No data\n").unwrap_err();
        assert!(matches!(
            err,
            Error::BadProviderData {
                provider: "stooq",
                ..
            }
        ));
    }

    #[test]
    fn skips_rows_without_price() {
        let csv = "Date,Open,High,Low,Close,Volume\n\
                   2024-06-03,192.90,194.99,192.52,194.03,50080539\n\
                   2024-06-04,-,-,-,-,0\n";
        let quotes = StooqProvider::parse_csv(&aapl(), csv).unwrap();
        assert_eq!(quotes.len(), 1);
    }

    #[test]
    fn builds_url_with_stooq_date_format() {
        let r = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        );
        assert_eq!(
            StooqProvider::url("AAPL.US", r),
            "https://stooq.com/q/d/l/?s=aapl.us&d1=20240601&d2=20240630&i=d"
        );
    }
}
