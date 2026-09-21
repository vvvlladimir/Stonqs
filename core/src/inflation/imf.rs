use super::{IndexPoint, IndexProvider, codes};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::model::normalize_region;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// The IMF's consumer price index, all items, 2010 = 100 — 189 economies plus Kosovo and the
/// West Bank and Gaza. It is the chain's second source rather than its first because it has no
/// euro-area aggregate and publishes later, but it is the only one that reaches beyond Europe.
pub struct ImfProvider {
    http_timeout: Duration,
}

impl Default for ImfProvider {
    fn default() -> Self {
        ImfProvider {
            http_timeout: Duration::from_secs(30),
        }
    }
}

impl ImfProvider {
    /// The stored name of this source (`price_index.source`).
    pub const ID: &'static str = "imf";

    pub fn new() -> Self {
        Self::default()
    }

    fn url(country: &str) -> String {
        // COUNTRY.INDEX_TYPE.COICOP.TYPE_OF_TRANSFORMATION.FREQUENCY — `_T` is all items,
        // `IX` the level rather than a rate the source derived from it.
        format!(
            "https://api.imf.org/external/sdmx/3.0/data/dataflow/IMF.STA/CPI/+/\
             {country}.CPI._T.IX.M?dimensionAtObservation=TIME_PERIOD"
        )
    }

    /// Reads SDMX-JSON 2.0. The key pins every dimension, so exactly one series is expected;
    /// anything else means the query stopped meaning what this code assumes it means.
    pub(crate) fn parse(region: &str, body: &str) -> Result<Vec<IndexPoint>> {
        let bad = |detail: String| Error::BadProviderData {
            provider: Self::ID,
            detail,
        };
        let json: serde_json::Value =
            serde_json::from_str(body).map_err(|e| bad(format!("malformed JSON: {e}")))?;

        let periods: Vec<NaiveDate> = json["data"]["structures"][0]["dimensions"]["observation"][0]["values"]
            .as_array()
            .ok_or_else(|| bad("no time dimension".into()))?
            .iter()
            .map(|v| {
                v["value"]
                    .as_str()
                    .or_else(|| v["id"].as_str())
                    .and_then(parse_month)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad("unreadable time period".into()))?;

        let series = json["data"]["dataSets"][0]["series"]
            .as_object()
            .ok_or_else(|| bad("no series".into()))?;
        if series.len() > 1 {
            return Err(bad(format!("{} series for one key", series.len())));
        }
        let Some((_, series)) = series.iter().next() else {
            return Ok(Vec::new());
        };
        let observations = series["observations"]
            .as_object()
            .ok_or_else(|| bad("no observations".into()))?;

        let mut points = Vec::new();
        for (slot, value) in observations {
            let (Ok(slot), Some(cell)) = (slot.parse::<usize>(), value.get(0)) else {
                continue;
            };
            let Some(month) = periods.get(slot) else { continue };
            // The IMF writes the figure as a JSON string of full precision, not a float.
            let Some(text) = cell
                .as_str()
                .map(str::to_owned)
                .or_else(|| cell.as_f64().map(|f| f.to_string()))
            else {
                continue;
            };
            if let Ok(level) = Decimal::from_str(&text).or_else(|_| Decimal::from_scientific(&text))
                && level > Decimal::ZERO
            {
                points.push(IndexPoint::new(region, *month, level));
            }
        }
        points.sort_by_key(|p| p.month);
        Ok(points)
    }
}

/// The IMF writes a month as `2025-M06`.
fn parse_month(period: &str) -> Option<NaiveDate> {
    let (year, month) = period.split_once("-M")?;
    NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, 1)
}

impl IndexProvider for ImfProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn covers(&self, region: &str) -> bool {
        codes::imf_country(&normalize_region(region)).is_some()
    }

    fn fetch(&self, region: &str, range: DateRange) -> Result<Vec<IndexPoint>> {
        let region = normalize_region(region);
        let Some(country) = codes::imf_country(&region) else {
            return Ok(Vec::new());
        };
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        // The source ignores a period filter and answers with the whole series; it is a few
        // hundred monthly figures, so the range is applied here instead.
        let body = agent
            .get(Self::url(country))
            .header("Accept", "application/vnd.sdmx.data+json")
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

    const BODY: &str = r#"{"data":{"dataSets":[{"series":{"0:0:0:0:0":{"observations":{
        "0":["152.7225866809138",null,0,"2010A",null],
        "1":["153.6870200476484",null,0,"2010A",null]}}}}],
        "structures":[{"dimensions":{"observation":[{"id":"TIME_PERIOD",
        "values":[{"value":"2025-M05"},{"value":"2025-M06"}]}]}}]}}"#;

    #[test]
    fn reads_one_level_per_month() {
        let points = ImfProvider::parse("US", BODY).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].month, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
        assert_eq!(points[1].value, dec!(153.6870200476484));
    }

    #[test]
    fn covers_beyond_europe_but_not_an_aggregate() {
        let p = ImfProvider::new();
        assert!(p.covers("RU") && p.covers("jp") && p.covers("XK"));
        assert!(!p.covers("EA") && !p.covers("EU"));
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_a_year_of_us_cpi() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let points = ImfProvider::new()
            .fetch("US", DateRange::new(d("2024-01-01"), d("2024-12-31")))
            .unwrap();
        assert_eq!(points.len(), 12);
    }
}
