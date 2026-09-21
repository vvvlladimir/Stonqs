use super::{Listing, ListingDirectory, mic};
use crate::error::{Error, Result};
use crate::model::is_isin;
use serde::{Deserialize, Serialize};
use std::time::Duration;
/// OpenFIGI listing directory; currencies and history come from quote sources.
pub struct OpenFigiDirectory {
    http_timeout: Duration,
    api_key: Option<String>,
}

impl Default for OpenFigiDirectory {
    fn default() -> Self {
        OpenFigiDirectory {
            http_timeout: Duration::from_secs(20),
            api_key: None,
        }
    }
}
/// OpenFIGI's anonymous and keyed batch limits.
const JOBS_PER_REQUEST_ANONYMOUS: usize = 10;
const JOBS_PER_REQUEST_WITH_KEY: usize = 100;

#[derive(Serialize)]
struct MappingJob<'a> {
    #[serde(rename = "idType")]
    id_type: &'a str,
    #[serde(rename = "idValue")]
    id_value: &'a str,
    #[serde(rename = "micCode")]
    mic_code: &'a str,
}

#[derive(Deserialize)]
struct MappingResult {
    #[serde(default)]
    data: Vec<MappingRow>,
    #[serde(default)]
    #[allow(dead_code)]
    warning: Option<String>,
}

#[derive(Deserialize)]
struct MappingRow {
    #[serde(default)]
    ticker: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

impl OpenFigiDirectory {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "openfigi";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    fn jobs_per_request(&self) -> usize {
        if self.api_key.is_some() {
            JOBS_PER_REQUEST_WITH_KEY
        } else {
            JOBS_PER_REQUEST_ANONYMOUS
        }
    }

    fn post(&self, body: String) -> Result<String> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let mut request = agent
            .post("https://api.openfigi.com/v3/mapping")
            .header("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            request = request.header("X-OPENFIGI-APIKEY", key);
        }
        Ok(request.send(body)?.body_mut().read_to_string()?)
    }
    /// Parse a response without network access; result order supplies each MIC.
    pub(crate) fn parse_batch(isin: &str, mics: &[&str], body: &str) -> Result<Vec<Listing>> {
        let results: Vec<MappingResult> = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: OpenFigiDirectory::ID,
            detail: format!("malformed JSON: {e}"),
        })?;
        if results.len() != mics.len() {
            return Err(Error::BadProviderData {
                provider: OpenFigiDirectory::ID,
                detail: format!("asked for {} listings, got {}", mics.len(), results.len()),
            });
        }

        let mut out = Vec::new();
        for (result, market) in results.into_iter().zip(mics) {
            for row in result.data {
                let Some(ticker) = row.ticker else {
                    continue;
                };
                if out
                    .iter()
                    .any(|l: &Listing| l.mic == *market && l.ticker == ticker)
                {
                    continue;
                }
                out.push(Listing {
                    isin: isin.to_string(),
                    mic: (*market).to_string(),
                    exchange: mic::market_name(market).map(str::to_string),
                    ticker,
                    name: row.name,
                    symbol: None,
                    currency: None,
                    has_history: None,
                    last_close: None,
                    source: OpenFigiDirectory::ID.to_string(),
                });
            }
        }
        Ok(out)
    }
}

impl ListingDirectory for OpenFigiDirectory {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn listings(&self, isin: &str) -> Result<Vec<Listing>> {
        // Reject non-ISIN input before a request whose empty result is ambiguous.
        if !is_isin(isin) {
            return Err(Error::Invalid(format!(
                "OpenFIGI looks listings up by ISIN, and {isin:?} does not look like one"
            )));
        }
        let isin = isin.trim().to_uppercase();

        let mics: Vec<&str> = mic::supported_mics().collect();
        let mut out = Vec::new();
        for chunk in mics.chunks(self.jobs_per_request()) {
            let jobs: Vec<MappingJob<'_>> = chunk
                .iter()
                .map(|m| MappingJob {
                    id_type: "ID_ISIN",
                    id_value: &isin,
                    mic_code: m,
                })
                .collect();
            let body = serde_json::to_string(&jobs).map_err(|e| Error::BadProviderData {
                provider: OpenFigiDirectory::ID,
                detail: e.to_string(),
            })?;
            out.extend(Self::parse_batch(&isin, chunk, &self.post(body)?)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
        {"data": [
            {"figi": "BBG000PPMQ13", "name": "ISHARES CORE MSCI WORLD",
             "ticker": "EUNL", "exchCode": "GT"},
            {"figi": "BBG000PPMQ22", "name": "ISHARES CORE MSCI WORLD",
             "ticker": "EUNL", "exchCode": "GT"}
        ]},
        {"warning": "No identifier found."}
    ]"#;

    #[test]
    fn reads_the_ticker_and_takes_the_mic_from_the_request() {
        let listings = OpenFigiDirectory::parse_batch("IE00B4L5Y983", &["XETR", "XTAE"], SAMPLE).unwrap();
        assert_eq!(listings.len(), 1, "two rows of one ticker are one listing");
        assert_eq!(listings[0].ticker, "EUNL");
        assert_eq!(listings[0].mic, "XETR");
        assert_eq!(listings[0].exchange.as_deref(), Some("Xetra"));
        assert_eq!(listings[0].currency, None, "OpenFIGI does not know the currency");
    }

    #[test]
    fn a_mismatched_batch_is_an_error() {
        let err = OpenFigiDirectory::parse_batch("IE00B4L5Y983", &["XETR"], "[]");
        assert!(err.is_err());
    }

    #[test]
    #[ignore = "requires network"]
    fn finds_the_german_listing_of_a_world_etf() {
        let found = OpenFigiDirectory::new().listings("IE00B4L5Y983").unwrap();
        let xetra = found.iter().find(|l| l.mic == "XETR").expect("a Xetra listing");
        assert_eq!(xetra.ticker, "EUNL");
        assert!(
            found.iter().any(|l| l.mic == "XLON"),
            "the London listing must be found as well"
        );
    }

    #[test]
    fn a_ticker_instead_of_an_isin_is_refused_before_the_request() {
        let err = OpenFigiDirectory::new().listings("IWDA.L").unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }
}
