use super::RateLookup;
use crate::error::Result;
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};

/// In-memory FX cache with forward-fill and inverse-pair lookup.
#[derive(Debug, Clone, Default)]
pub struct RateCache {
    series: HashMap<(Currency, Currency), BTreeMap<NaiveDate, Decimal>>,
}

impl RateCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_series(&mut self, from: &str, to: &str, series: BTreeMap<NaiveDate, Decimal>) {
        self.series
            .insert((normalize_currency(from), normalize_currency(to)), series);
    }

    fn direct(&self, from: &Currency, to: &Currency, date: NaiveDate) -> Option<Decimal> {
        self.series
            .get(&(from.clone(), to.clone()))
            .and_then(|s| s.range(..=date).next_back())
            .map(|(_, v)| *v)
    }
}

impl RateLookup for RateCache {
    fn rate_as_of(&self, from: &str, to: &str, date: NaiveDate) -> Result<Option<Decimal>> {
        let (from, to) = (normalize_currency(from), normalize_currency(to));
        if from == to {
            return Ok(Some(Decimal::ONE));
        }
        if let Some(r) = self.direct(&from, &to, date) {
            return Ok(Some(r));
        }
        match self.direct(&to, &from, date) {
            Some(r) if !r.is_zero() => Ok(Some(Decimal::ONE / r)),
            _ => Ok(None),
        }
    }
}
