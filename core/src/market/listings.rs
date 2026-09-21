//! Listing-directory types; a listing is enriched by a quote provider later.

use crate::error::Result;
use crate::money::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
/// A venue-specific listing that may not have been probed yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listing {
    pub isin: String,
    /// ISO 10383 market identifier.
    pub mic: String,
    pub ticker: String,
    pub exchange: Option<String>,
    pub name: Option<String>,
    /// Provider symbol, if the quote provider can construct one.
    pub symbol: Option<String>,
    pub currency: Option<Currency>,
    /// Whether the provider returned historical candles for `symbol`.
    pub has_history: Option<bool>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub last_close: Option<Decimal>,
    pub source: String,
}
impl Listing {
    /// True only when the listing has a symbol, currency, and verified history.
    pub fn is_usable(&self) -> bool {
        self.symbol.is_some() && self.has_history == Some(true) && self.currency.is_some()
    }
}
/// Source of venue and ticker mappings for an ISIN.
pub trait ListingDirectory: Send + Sync {
    fn id(&self) -> &'static str;

    fn listings(&self, isin: &str) -> Result<Vec<Listing>>;
}
