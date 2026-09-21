use super::{PriceLookup, PricePoint};
use crate::error::Result;
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap};
/// In-memory equivalent of the store's backward-filled price lookup.
#[derive(Debug, Clone, Default)]
pub struct PriceCache {
    series: HashMap<String, BTreeMap<NaiveDate, PricePoint>>,
}

impl PriceCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_series(&mut self, security_id: &str, series: BTreeMap<NaiveDate, PricePoint>) {
        self.series.insert(security_id.to_string(), series);
    }
    /// Whether a series was loaded for the security.
    pub fn has(&self, security_id: &str) -> bool {
        self.series.contains_key(security_id)
    }
}

impl PriceLookup for PriceCache {
    fn price_as_of(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        Ok(self
            .series
            .get(security_id)
            .and_then(|s| s.range(..=date).next_back())
            .map(|(_, v)| v.clone()))
    }

    fn price_before(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        Ok(self.series.get(security_id).and_then(|s| {
            // Select the effective quote first, then step back in the series.
            let (effective, _) = s.range(..=date).next_back()?;
            s.range(..*effective).next_back().map(|(_, v)| v.clone())
        }))
    }
}
