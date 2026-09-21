use super::{FxProvider, FxRate};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::money::normalize_currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

/// What the ECB publishes today, EUR included. RUB stopped on 2022-03-01, HRK with the euro.
pub(crate) const PUBLISHED: &[&str] = &[
    "EUR", "USD", "JPY", "BGN", "CZK", "DKK", "GBP", "HUF", "PLN", "RON", "SEK", "CHF", "ISK", "NOK", "TRY",
    "AUD", "BRL", "CAD", "CNY", "HKD", "IDR", "ILS", "INR", "KRW", "MXN", "MYR", "NZD", "PHP", "SGD", "THB",
    "ZAR",
];

/// ECB reference-rate provider; see ADR 0001 for the source decision.
pub struct EcbProvider {
    http_timeout: Duration,
}

impl Default for EcbProvider {
    fn default() -> Self {
        EcbProvider {
            http_timeout: Duration::from_secs(20),
        }
    }
}

impl EcbProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "ecb";

    pub fn new() -> Self {
        Self::default()
    }

    fn url(currency: &str, range: DateRange) -> String {
        format!(
            "https://data-api.ecb.europa.eu/service/data/EXR/D.{currency}.EUR.SP00.A\
             ?startPeriod={}&endPeriod={}&format=csvdata",
            range.from, range.to
        )
    }

    /// Parses the ECB CSV response into a date-indexed series.
    pub(crate) fn parse_csv(body: &str) -> Result<BTreeMap<NaiveDate, Decimal>> {
        let mut lines = body.lines();
        let header = lines.next().unwrap_or_default();
        if !header.starts_with("KEY,") {
            return Err(Error::BadProviderData {
                provider: EcbProvider::ID,
                detail: format!("unexpected header: {header:?}"),
            });
        }

        let mut series = BTreeMap::new();
        for line in lines {
            // The needed ECB fields precede quoted fields; see ADR 0001.
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 8 {
                continue;
            }
            let Ok(date) = NaiveDate::parse_from_str(cols[6], "%Y-%m-%d") else {
                continue;
            };

            let Ok(rate) = Decimal::from_str(cols[7]) else {
                continue;
            };
            if rate > Decimal::ZERO {
                series.insert(date, rate);
            }
        }
        Ok(series)
    }

    fn fetch_eur_series(&self, currency: &str, range: DateRange) -> Result<BTreeMap<NaiveDate, Decimal>> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let body = agent
            .get(Self::url(currency, range))
            .call()?
            .body_mut()
            .read_to_string()?;
        Self::parse_csv(&body)
    }
}

impl FxProvider for EcbProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn covers(&self, currency: &str) -> bool {
        PUBLISHED.contains(&normalize_currency(currency).as_str())
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        if base == quote {
            return Ok(Vec::new());
        }

        let series: BTreeMap<NaiveDate, Decimal> = if base == "EUR" {
            self.fetch_eur_series(&quote, range)?
        } else if quote == "EUR" {
            self.fetch_eur_series(&base, range)?
                .into_iter()
                .map(|(d, r)| (d, Decimal::ONE / r))
                .collect()
        } else {
            let from_eur_to_base = self.fetch_eur_series(&base, range)?;
            let from_eur_to_quote = self.fetch_eur_series(&quote, range)?;

            // Cross-rates require both legs on the same date.
            from_eur_to_base
                .iter()
                .filter_map(|(d, base_rate)| {
                    from_eur_to_quote
                        .get(d)
                        .map(|quote_rate| (*d, quote_rate / base_rate))
                })
                .collect()
        };

        Ok(series
            .into_iter()
            .map(|(date, rate)| FxRate::new(&base, &quote, date, rate))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_only_what_it_still_publishes() {
        let ecb = EcbProvider::new();
        assert!(ecb.covers("usd") && ecb.covers("EUR"));
        assert!(!ecb.covers("RUB") && !ecb.covers("ARS"));
    }
    use rust_decimal_macros::dec;

    const SAMPLE: &str = "KEY,FREQ,CURRENCY,CURRENCY_DENOM,EXR_TYPE,EXR_SUFFIX,TIME_PERIOD,OBS_VALUE,OBS_STATUS\n\
        EXR.D.USD.EUR.SP00.A,D,USD,EUR,SP00,A,2024-06-03,1.0842,A\n\
        EXR.D.USD.EUR.SP00.A,D,USD,EUR,SP00,A,2024-06-04,1.0865,A\n\
        EXR.D.USD.EUR.SP00.A,D,USD,EUR,SP00,A,2024-06-05,,A\n";

    #[test]
    fn parses_ecb_csv_and_skips_empty_observations() {
        let series = EcbProvider::parse_csv(SAMPLE).unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(
            series[&NaiveDate::from_ymd_opt(2024, 6, 3).unwrap()],
            dec!(1.0842)
        );
        assert_eq!(
            series[&NaiveDate::from_ymd_opt(2024, 6, 4).unwrap()],
            dec!(1.0865)
        );
    }

    #[test]
    fn rejects_unexpected_body() {
        assert!(matches!(
            EcbProvider::parse_csv("<html>maintenance</html>"),
            Err(Error::BadProviderData { provider: "ecb", .. })
        ));
    }

    #[test]
    #[ignore = "requires network"]
    fn ecb_live() {
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2024, 6, 3).unwrap(),
            NaiveDate::from_ymd_opt(2024, 6, 7).unwrap(),
        );
        let rates = EcbProvider::new().fetch("USD", "EUR", range).unwrap();
        assert!(!rates.is_empty());

        assert!(rates.iter().all(|r| r.rate > dec!(0.5) && r.rate < dec!(1.5)));
    }
}
