//! Moving an instrument off a venue its source barely quotes. Only in `RefreshMode::Missing`:
//! a venue the user chose by hand is never overwritten unasked.

use super::window::sparse_history;
use chrono::{Duration, NaiveDate};
use sq_core::market::{Listing, MarketDataService};
use sq_core::prelude::*;
use sq_core::storage::Store;

/// `None` when nothing changed; `Some(n)` with the quotes the new venue gave (possibly zero).
pub(super) fn relist(
    store: &Store,
    quotes: &MarketDataService,
    security: &Security,
    held_from: Option<NaiveDate>,
    today: NaiveDate,
    settled_through: NaiveDate,
) -> Option<usize> {
    if security.data_source.is_none() || !security.is_quotable() {
        return None;
    }
    let span = store.quote_span(&security.id).ok().flatten();
    if !sparse_history(held_from, span, today) {
        return None;
    }
    let found = match better_listing(quotes, security) {
        Ok(Some(found)) => found,
        // The directory answered and no venue has prices: stop asking at every import.
        Ok(None) => return retire(store, security, span),
        // Not reached: that is the network, not an answer.
        Err(_) => return None,
    };
    let symbol = found.symbol.clone()?.to_uppercase();
    if symbol == security.provider_symbol().to_uppercase() {
        return None;
    }

    let updated = Security {
        symbol,
        currency: found
            .currency
            .clone()
            .unwrap_or_else(|| security.currency.clone()),
        data_source: Some(found.source.clone()),
        data_symbol: None,
        mic: Some(found.mic.clone()).filter(|m| !m.is_empty()),
        ..security.clone()
    };
    // The old series belonged to the old ticker and its currency.
    store.delete_quotes(&updated.id).ok()?;
    store.save_security(&updated).ok()?;
    let range = DateRange::new(
        held_from.unwrap_or_else(|| today - Duration::days(365 * 5)),
        today,
    );
    Some(
        quotes
            .ensure_history_through(store, &updated, range, settled_through)
            .unwrap_or(0),
    )
}

/// By ISIN through the directory, else by the bare ticker. `Ok(None)` = answered with nothing.
fn better_listing(quotes: &MarketDataService, security: &Security) -> Result<Option<Listing>> {
    let preferred = Some(security.currency.as_str());
    match security.isin.as_deref().filter(|i| sq_core::model::is_isin(i)) {
        Some(isin) => quotes.best_listing(isin, preferred),
        None => quotes.best_listing_by_symbol(security.provider_symbol(), preferred),
    }
}

/// Manual prices from here on — only when the source returned nothing at all; a thin series
/// still has prices worth keeping.
fn retire(store: &Store, security: &Security, span: Option<DateRange>) -> Option<usize> {
    if span.is_some() {
        return None;
    }
    let updated = Security {
        data_source: None,
        ..security.clone()
    };
    store.save_security(&updated).ok()?;
    Some(0)
}
