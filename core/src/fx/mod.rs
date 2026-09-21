//! Historical FX rates and as-of conversion.
//! Lookup mirrors market prices: identity, inverse pairs, and backward fill.

mod cache;
mod ecb;
mod frankfurter;
mod provider;
mod service;
mod yahoo;

pub use cache::RateCache;
pub use ecb::EcbProvider;
pub use frankfurter::FrankfurterProvider;
pub use provider::{FxProvider, StaticFxProvider};
pub use service::FxService;
pub use yahoo::YahooFxProvider;

use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An FX rate quoted as units of `quote` per one unit of `base`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FxRate {
    pub base: Currency,
    pub quote: Currency,
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub rate: Decimal,
}

impl FxRate {
    pub fn new(base: &str, quote: &str, date: NaiveDate, rate: Decimal) -> Self {
        FxRate {
            base: normalize_currency(base),
            quote: normalize_currency(quote),
            date,
            rate,
        }
    }
}
/// Looks up rates as of a date and converts money.
pub trait RateLookup {
    /// Returns the latest known rate on or before `date`.
    fn rate_as_of(&self, from: &str, to: &str, date: NaiveDate) -> crate::error::Result<Option<Decimal>>;

    /// Converts an amount or returns `MissingMarketData` when no rate exists.
    fn convert(
        &self,
        amount: Decimal,
        from: &str,
        to: &str,
        date: NaiveDate,
    ) -> crate::error::Result<Decimal> {
        let rate =
            self.rate_as_of(from, to, date)?
                .ok_or_else(|| crate::error::Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{}", normalize_currency(from), normalize_currency(to)),
                    date,
                })?;
        Ok(amount * rate)
    }
}
