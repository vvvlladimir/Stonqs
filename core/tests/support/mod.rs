//! Deterministic market-data doubles for calculation-engine tests.

#![allow(dead_code)]

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sq_core::error::Result;
use sq_core::fx::RateLookup;
use sq_core::market::{PriceLookup, PricePoint};
use std::collections::BTreeMap;

/// Quote currency is explicit; it must not be inferred from the position.
pub struct FakePrices {
    default_currency: String,
    series: BTreeMap<String, BTreeMap<NaiveDate, PricePoint>>,
}

impl FakePrices {
    pub fn new(default_currency: &str) -> Self {
        FakePrices {
            default_currency: default_currency.to_string(),
            series: BTreeMap::new(),
        }
    }

    pub fn with(self, security_id: &str, date: NaiveDate, price: Decimal) -> Self {
        let currency = self.default_currency.clone();
        self.with_in(security_id, date, price, &currency)
    }

    /// Price quoted in a currency different from the other holdings.
    pub fn with_in(mut self, security_id: &str, date: NaiveDate, price: Decimal, currency: &str) -> Self {
        self.series
            .entry(security_id.to_string())
            .or_default()
            .insert(date, PricePoint::new(price, currency));
        self
    }
}

impl PriceLookup for FakePrices {
    fn price_as_of(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        Ok(self
            .series
            .get(security_id)
            // Select the latest quote not after `date`, matching SQL forward-fill.
            .and_then(|s| s.range(..=date).next_back())
            .map(|(_, v)| v.clone()))
    }

    fn price_before(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        Ok(self.series.get(security_id).and_then(|s| {
            // Step back from the active quote, matching `PriceCache`.
            let (effective, _) = s.range(..=date).next_back()?;
            s.range(..*effective).next_back().map(|(_, v)| v.clone())
        }))
    }
}

#[derive(Default)]
pub struct FakeRates {
    series: BTreeMap<(String, String), BTreeMap<NaiveDate, Decimal>>,
}

impl FakeRates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, from: &str, to: &str, date: NaiveDate, rate: Decimal) -> Self {
        self.series
            .entry((from.to_string(), to.to_string()))
            .or_default()
            .insert(date, rate);
        self
    }
}

impl RateLookup for FakeRates {
    fn rate_as_of(&self, from: &str, to: &str, date: NaiveDate) -> Result<Option<Decimal>> {
        if from == to {
            return Ok(Some(Decimal::ONE));
        }
        if let Some(r) = self
            .series
            .get(&(from.to_string(), to.to_string()))
            .and_then(|s| s.range(..=date).next_back())
        {
            return Ok(Some(*r.1));
        }
        Ok(self
            .series
            .get(&(to.to_string(), from.to_string()))
            .and_then(|s| s.range(..=date).next_back())
            .map(|(_, r)| Decimal::ONE / r))
    }
}

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
