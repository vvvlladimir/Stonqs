use super::{FxProvider, FxRate};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::money::normalize_currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// Frankfurter republishes the ECB's reference rates from another host: the same currencies and
/// the same figures, so it stands in when the ECB itself cannot be reached.
pub struct FrankfurterProvider {
    http_timeout: Duration,
}

impl Default for FrankfurterProvider {
    fn default() -> Self {
        FrankfurterProvider {
            http_timeout: Duration::from_secs(20),
        }
    }
}

impl FrankfurterProvider {
    /// The stored name of this source (`fx_rates.source`).
    pub const ID: &'static str = "frankfurter";

    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse(base: &str, quote: &str, body: &str) -> Result<Vec<FxRate>> {
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: Self::ID,
            detail: format!("malformed JSON: {e}"),
        })?;
        let Some(days) = json["rates"].as_object() else {
            return Err(Error::BadProviderData {
                provider: Self::ID,
                detail: "no rates".into(),
            });
        };
        let mut rates = Vec::new();
        for (day, values) in days {
            let (Ok(date), Some(rate)) = (NaiveDate::parse_from_str(day, "%Y-%m-%d"), values.get(quote))
            else {
                continue;
            };
            // serde_json prints an f64 back in its shortest form, which is the decimal the source wrote.
            let text = rate.to_string();
            if let Ok(rate) = Decimal::from_str(&text).or_else(|_| Decimal::from_scientific(&text))
                && rate > Decimal::ZERO
            {
                rates.push(FxRate::new(base, quote, date, rate));
            }
        }
        rates.sort_by_key(|r| r.date);
        Ok(rates)
    }
}

impl FxProvider for FrankfurterProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn covers(&self, currency: &str) -> bool {
        super::ecb::PUBLISHED.contains(&normalize_currency(currency).as_str())
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        if base == quote {
            return Ok(Vec::new());
        }
        let url = format!(
            "https://api.frankfurter.dev/v1/{}..{}?base={base}&symbols={quote}",
            range.from, range.to
        );
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let body = agent.get(url).call()?.body_mut().read_to_string()?;
        // Asked from a weekend, Frankfurter answers with the Friday before; keep only the range.
        Ok(Self::parse(&base, &quote, &body)?
            .into_iter()
            .filter(|r| r.date >= range.from && r.date <= range.to)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn reads_one_rate_per_day() {
        let body = r#"{"amount":1.0,"base":"USD","rates":{"2024-06-04":{"EUR":0.92039},"2024-06-03":{"EUR":0.92234}}}"#;
        let rates = FrankfurterProvider::parse("USD", "EUR", body).unwrap();
        assert_eq!(rates.len(), 2);
        assert_eq!(rates[0].rate, dec!(0.92234));
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_a_week_of_usd_eur() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let rates = FrankfurterProvider::new()
            .fetch("USD", "EUR", DateRange::new(d("2024-06-03"), d("2024-06-07")))
            .unwrap();
        assert_eq!(rates.len(), 5);
    }
}
