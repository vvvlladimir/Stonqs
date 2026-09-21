use super::{IndexPoint, IndexProvider, codes};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::model::normalize_region;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// Every geography Eurostat publishes the harmonised index for, in this app's own vocabulary.
/// `EA` and `EU` are the aggregates; `US` is here because Eurostat computes a harmonised series
/// for it, which is not the same thing as the American CPI the IMF carries.
pub(crate) const PUBLISHED: &[&str] = &[
    "EA", "EU", "AL", "AT", "BE", "BG", "CH", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GB", "GE",
    "GR", "HR", "HU", "IE", "IS", "IT", "LT", "LU", "LV", "ME", "MK", "MT", "NL", "NO", "PL", "PT", "RO",
    "RS", "SE", "SI", "SK", "TR", "US", "XK",
];

/// Eurostat's harmonised index of consumer prices, all items, 2015 = 100. Preferred over the IMF
/// for the geographies it covers: it publishes a euro-area aggregate, which the IMF does not, and
/// it publishes sooner — around the middle of the following month.
///
/// The dataset is the ECOICOP ver. 2 one (`prc_hicp_minr`, classification dimension `coicop18`).
/// Its ver. 1 predecessor `prc_hicp_midx` still answers every request and still returns HTTP 200,
/// but it stopped being updated at the end of 2025 — which is why the classification version is
/// named in this comment: a retired Eurostat dataset goes stale silently rather than erroring,
/// and nothing in a chain that only falls through on failure would ever notice.
pub struct EurostatProvider {
    http_timeout: Duration,
}

impl Default for EurostatProvider {
    fn default() -> Self {
        EurostatProvider {
            http_timeout: Duration::from_secs(20),
        }
    }
}

impl EurostatProvider {
    /// The stored name of this source (`price_index.source`).
    pub const ID: &'static str = "eurostat";

    pub fn new() -> Self {
        Self::default()
    }

    fn url(geo: &str) -> String {
        format!(
            "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/prc_hicp_minr\
             ?format=JSON&lang=en&unit=I15&coicop18=TOTAL&freq=M&geo={geo}"
        )
    }

    /// Reads JSON-stat 2.0. Every dimension but time is pinned to one value by the query, so a
    /// flat value key is a time index — which the response is asked to confirm rather than
    /// assumed, because a silently widened dimension would misdate the whole series.
    pub(crate) fn parse(region: &str, body: &str) -> Result<Vec<IndexPoint>> {
        let bad = |detail: String| Error::BadProviderData {
            provider: Self::ID,
            detail,
        };
        let json: serde_json::Value =
            serde_json::from_str(body).map_err(|e| bad(format!("malformed JSON: {e}")))?;

        let ids = json["id"]
            .as_array()
            .ok_or_else(|| bad("no dimension order".into()))?;
        let sizes = json["size"].as_array().ok_or_else(|| bad("no sizes".into()))?;
        for (id, size) in ids.iter().zip(sizes) {
            if id.as_str() != Some("time") && size.as_u64() != Some(1) {
                return Err(bad(format!("dimension {id} is not pinned to one value")));
            }
        }

        let months = json["dimension"]["time"]["category"]["index"]
            .as_object()
            .ok_or_else(|| bad("no time index".into()))?;
        let values = json["value"].as_object().ok_or_else(|| bad("no values".into()))?;

        let mut points = Vec::new();
        for (period, slot) in months {
            let Some(slot) = slot.as_u64() else { continue };
            let Some(value) = values.get(slot.to_string().as_str()) else {
                continue;
            };
            let Some(month) = parse_month(period) else {
                continue;
            };
            // serde_json prints an f64 back in its shortest form, which is the decimal Eurostat wrote.
            let text = value.to_string();
            if let Ok(level) = Decimal::from_str(&text).or_else(|_| Decimal::from_scientific(&text))
                && level > Decimal::ZERO
            {
                points.push(IndexPoint::new(region, month, level));
            }
        }
        points.sort_by_key(|p| p.month);
        Ok(points)
    }
}

/// Eurostat writes a month as `2025-06`.
fn parse_month(period: &str) -> Option<NaiveDate> {
    let (year, month) = period.split_once('-')?;
    NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, 1)
}

impl IndexProvider for EurostatProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn covers(&self, region: &str) -> bool {
        PUBLISHED.contains(&normalize_region(region).as_str())
    }

    fn fetch(&self, region: &str, range: DateRange) -> Result<Vec<IndexPoint>> {
        let region = normalize_region(region);
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        let body = agent
            .get(Self::url(&codes::eurostat_geo(&region)))
            .call()?
            .body_mut()
            .read_to_string()?;
        let from = super::first_of_month(range.from);
        Ok(Self::parse(&region, &body)?
            .into_iter()
            .filter(|p| p.month >= from && p.month <= range.to)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const BODY: &str = r#"{"version":"2.0","class":"dataset","value":{"0":128.5,"1":129.09},
        "id":["freq","unit","coicop18","geo","time"],"size":[1,1,1,1,2],
        "dimension":{"time":{"category":{"index":{"2025-05":0,"2025-06":1}}}}}"#;

    #[test]
    fn reads_one_level_per_month() {
        let points = EurostatProvider::parse("EA", BODY).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].month, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
        assert_eq!(points[1].value, dec!(129.09));
    }

    #[test]
    fn refuses_a_response_whose_dimensions_widened() {
        let body = BODY.replace(r#""size":[1,1,1,1,2]"#, r#""size":[1,1,1,2,2]"#);
        assert!(EurostatProvider::parse("EA", &body).is_err());
    }

    #[test]
    fn covers_the_aggregates_and_not_the_world() {
        let p = EurostatProvider::new();
        assert!(p.covers("ea") && p.covers("DE") && p.covers("US"));
        assert!(!p.covers("RU") && !p.covers("JP"));
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_the_euro_area() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let points = EurostatProvider::new()
            .fetch("EA", DateRange::new(d("2024-01-01"), d("2024-12-31")))
            .unwrap();
        assert_eq!(points.len(), 12);
    }

    /// Guards the dataset choice rather than the parser: the retired one answers 200 with a
    /// series that simply stops, so only asking for a recent month catches the swap.
    #[test]
    #[ignore = "requires network"]
    fn the_series_reaches_the_month_before_last() {
        let today = chrono::Utc::now().date_naive();
        let want = super::super::first_of_month(today) - chrono::Months::new(2);
        let points = EurostatProvider::new()
            .fetch("DE", DateRange::new(want, today))
            .unwrap();
        assert!(
            points.iter().any(|p| p.month >= want),
            "Germany stops before {want}: the dataset has most likely been retired"
        );
    }
}
