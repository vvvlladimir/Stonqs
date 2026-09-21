use super::FxRate;
use crate::error::Result;
use crate::market::DateRange;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Source of historical FX rates.
pub trait FxProvider: Send + Sync {
    fn id(&self) -> &'static str;

    /// Fetches one base/quote series for the requested date range.
    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>>;

    /// Whether this source publishes `currency` at all. A chain skips a source that does not,
    /// so an empty answer from one that does (a weekend) is final rather than a cue to ask on.
    fn covers(&self, _currency: &str) -> bool {
        true
    }
}

/// Deterministic provider used by tests and offline workflows.
#[derive(Debug, Default, Clone)]
pub struct StaticFxProvider {
    rates: BTreeMap<(String, String), BTreeMap<NaiveDate, Decimal>>,
}
/// Builder-style insertion helper.
impl StaticFxProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, base: &str, quote: &str, date: NaiveDate, rate: Decimal) {
        let key = (
            crate::money::normalize_currency(base),
            crate::money::normalize_currency(quote),
        );
        self.rates.entry(key).or_default().insert(date, rate);
    }

    pub fn with(mut self, base: &str, quote: &str, date: NaiveDate, rate: Decimal) -> Self {
        self.insert(base, quote, date, rate);
        self
    }
}

impl FxProvider for StaticFxProvider {
    fn id(&self) -> &'static str {
        "static"
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let key = (
            crate::money::normalize_currency(base),
            crate::money::normalize_currency(quote),
        );
        let Some(series) = self.rates.get(&key) else {
            return Ok(Vec::new());
        };
        Ok(series
            .range(range.from..=range.to)
            .map(|(d, r)| FxRate::new(&key.0, &key.1, *d, *r))
            .collect())
    }
}
