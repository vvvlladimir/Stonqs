//! Market quotes, provider seams, listing resolution, and as-of lookup.

mod cache;
mod custom;
mod eodhd;
mod guard;
mod kraken;
mod listings;
pub mod mic;
mod openfigi;
mod provider;
mod retry;
mod search;
mod service;
mod stooq;
mod twelvedata;
mod yahoo;

pub use cache::PriceCache;
pub use custom::{CUSTOM_PREFIX, CustomFormat, CustomHeader, CustomProvider, CustomRole, CustomSource};
pub use eodhd::EodhdProvider;
pub use kraken::KrakenProvider;
pub use listings::{Listing, ListingDirectory};
pub use openfigi::OpenFigiDirectory;

pub use provider::{DateRange, History, QuoteProvider};
pub use retry::{Budgets, FetchPolicy};
pub use search::{SecurityMatch, SecuritySearch};
pub use service::MarketDataService;
pub use stooq::StooqProvider;
pub use twelvedata::TwelveDataProvider;
pub use yahoo::YahooProvider;

use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
/// One daily closing price, paired with the currency returned by its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub security_id: String,
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub close: Decimal,
    pub currency: Currency,
    pub source: String,
}
/// A quote value used by calculation code and in-memory lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricePoint {
    #[serde(with = "rust_decimal::serde::str")]
    pub close: Decimal,
    pub currency: Currency,
}

impl PricePoint {
    pub fn new(close: Decimal, currency: impl AsRef<str>) -> Self {
        PricePoint {
            close,
            currency: crate::money::normalize_currency(currency.as_ref()),
        }
    }
}
/// Price lookup with backward fill; it never uses a quote from the future.
pub trait PriceLookup {
    /// Return the quote on `date`, or the latest quote before it.
    fn price_as_of(&self, security_id: &str, date: NaiveDate) -> crate::error::Result<Option<PricePoint>>;

    /// Return the quote preceding the one selected by `price_as_of`.
    fn price_before(&self, security_id: &str, date: NaiveDate) -> crate::error::Result<Option<PricePoint>>;
}
