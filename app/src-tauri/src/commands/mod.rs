//! Tauri commands exposed to the frontend.

pub mod accounts;
pub mod ai;
pub mod alerts;
pub mod allocation;
pub mod attributes;
pub mod corporate_actions;
pub mod dashboard;
pub mod demo;
pub mod dev;
pub mod import;
pub mod inflation;
pub mod listings;
pub mod lookup;
pub mod payments;
pub mod performance;
pub mod periods;
pub mod plans;
pub mod portfolio;
pub mod positions;
pub mod profiles;
pub mod reports;
pub mod securities;
pub mod sources;
pub mod trades;
pub mod transactions;
pub mod watchlists;

use crate::error::{UiError, UiResult};
use chrono::NaiveDate;
use sq_core::storage::Store;

/// Parse the single `YYYY-MM-DD` date format used across the wire boundary.
pub(crate) fn parse_date(value: &str) -> UiResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|e| UiError::invalid(format!("invalid date {value:?}: {e}")))
}

/// Replace a security ID with a readable symbol in market-data errors.
pub(crate) fn named(store: &Store, error: sq_core::Error) -> UiError {
    let sq_core::Error::MissingMarketData { kind, key, date } = &error else {
        return error.into();
    };
    let Ok(security) = store.get_security(key) else {
        return error.into();
    };
    UiError::MissingMarketData {
        kind: kind.to_string(),
        key: format!("{} · {}", security.symbol, security.name),
        date: date.to_string(),
        message: format!(
            "no price for {} ({}) on {date} or earlier",
            security.symbol, security.name
        ),
    }
}
