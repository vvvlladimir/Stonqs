use super::IndexPoint;
use crate::error::Result;
use crate::market::DateRange;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Source of consumer-price index levels.
pub trait IndexProvider: Send + Sync {
    fn id(&self) -> &'static str;

    /// Levels for one region over the requested range, oldest first.
    fn fetch(&self, region: &str, range: DateRange) -> Result<Vec<IndexPoint>>;

    /// Whether this source publishes `region` at all. A chain skips one that does not, so an
    /// empty answer from one that does is final rather than a cue to ask on.
    fn covers(&self, _region: &str) -> bool {
        true
    }
}

/// Deterministic provider used by tests and offline workflows.
#[derive(Debug, Default, Clone)]
pub struct StaticIndexProvider {
    levels: BTreeMap<String, BTreeMap<NaiveDate, Decimal>>,
}

impl StaticIndexProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, region: &str, month: NaiveDate, value: Decimal) {
        let point = IndexPoint::new(region, month, value);
        self.levels
            .entry(point.region)
            .or_default()
            .insert(point.month, point.value);
    }

    pub fn with(mut self, region: &str, month: NaiveDate, value: Decimal) -> Self {
        self.insert(region, month, value);
        self
    }
}

impl IndexProvider for StaticIndexProvider {
    fn id(&self) -> &'static str {
        "static"
    }

    fn fetch(&self, region: &str, range: DateRange) -> Result<Vec<IndexPoint>> {
        let region = crate::model::normalize_region(region);
        let Some(series) = self.levels.get(&region) else {
            return Ok(Vec::new());
        };
        Ok(series
            .range(super::first_of_month(range.from)..=range.to)
            .map(|(month, value)| IndexPoint::new(&region, *month, *value))
            .collect())
    }

    fn covers(&self, region: &str) -> bool {
        self.levels.contains_key(&crate::model::normalize_region(region))
    }
}
