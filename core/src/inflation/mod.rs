//! Consumer-price index levels and the real returns derived from them.
//!
//! Mirrors `fx/`: one-method providers, a service that caches and chains them, and a lookup
//! trait that keeps `calc` testable without a network. What differs is the grain — an index is
//! published once a month, so a lookup steps rather than interpolates: a daily inflation rate
//! is not a fact anybody published.

mod codes;
mod eurostat;
mod imf;
mod provider;
mod service;

pub use eurostat::EurostatProvider;
pub use imf::ImfProvider;
pub use provider::{IndexProvider, StaticIndexProvider};
pub use service::InflationService;

use crate::error::Result;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One published index level. `month` is the month's first day, so it orders and stores like
/// every other date here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexPoint {
    pub region: String,
    pub month: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub value: Decimal,
}

impl IndexPoint {
    pub fn new(region: &str, month: NaiveDate, value: Decimal) -> Self {
        IndexPoint {
            region: crate::model::normalize_region(region),
            month: first_of_month(month),
            value,
        }
    }
}

/// Every region this build can fetch an index for, sorted — the union of what its sources
/// publish. It carries codes only: which of them is worth offering, and under what name, is the
/// frontend's business, where the language is known.
pub fn regions() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = eurostat::PUBLISHED
        .iter()
        .copied()
        .chain(codes::alpha2())
        .collect();
    all.sort_unstable();
    all.dedup();
    all
}

/// The first day of `date`'s month — the key every index level is stored under.
pub fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

/// Reads index levels as of a date. Backward fill only, like every other lookup here.
pub trait IndexLookup {
    /// The latest level published on or before `date`.
    fn index_as_of(&self, region: &str, date: NaiveDate) -> Result<Option<Decimal>>;

    /// The last month `region` has a level for. A window cannot be deflated past it, and the
    /// gap is normal rather than an error: a month's index is published weeks after it ends.
    fn index_through(&self, region: &str) -> Result<Option<NaiveDate>>;
}
