//! Search and profile interfaces for resolving provider symbols to securities.

use crate::error::Result;
use crate::model::SecurityKind;
use crate::money::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
/// Search output before it is persisted as a `Security`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityMatch {
    pub source: String,
    pub symbol: String,
    pub name: String,
    pub exchange: Option<String>,
    /// ISO 10383 code of the venue the source quotes this symbol on, when it names one.
    pub mic: Option<String>,
    pub kind: SecurityKind,
    /// `None` means the search endpoint did not provide currency metadata.
    pub currency: Option<Currency>,
    pub isin: Option<String>,
    /// `None` means history has not been probed yet.
    pub has_history: Option<bool>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub last_close: Option<Decimal>,
}

impl SecurityMatch {
    /// Whether the match has enough metadata for a usable security.
    pub fn is_complete(&self) -> bool {
        self.currency.is_some() && self.has_history != Some(false)
    }
}
/// Search, profile, and optional venue-to-symbol conversion for one source.
pub trait SecuritySearch: Send + Sync {
    fn id(&self) -> &'static str;

    fn search(&self, query: &str) -> Result<Vec<SecurityMatch>>;

    fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>>;

    fn symbol_for(&self, _ticker: &str, _mic: &str) -> Option<String> {
        None
    }

    /// Inverse of `symbol_for`: which venue this source quotes `symbol` on. `exchange` is the
    /// source's own display name, the only signal left once the symbol carries no venue suffix.
    fn mic_for(&self, _symbol: &str, _exchange: Option<&str>) -> Option<&'static str> {
        None
    }
}
