use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use serde::Deserialize;
use sq_core::market::Listing;
use sq_core::model::is_isin;
use sq_core::prelude::*;
use tauri::{AppHandle, State};

const PROBE_LIMIT: usize = 8;

#[tauri::command]
pub async fn security_listings(
    state: State<'_, AppState>,
    id: String,
    refresh: bool,
) -> UiResult<Vec<Listing>> {
    let security = state.store()?.get_security(&id)?;
    let setup = state.market_setup();
    // Without an ISIN the directory cannot be asked at all, so the quote source is asked for the
    // ticker instead. That answer is not cached: the listings table is keyed by ISIN.
    let Some(isin) = security.isin.clone().filter(|i| is_isin(i)) else {
        let symbol = security.provider_symbol().to_string();
        return tauri::async_runtime::spawn_blocking(move || {
            let service = sq_core::sources::quote_service_with(&setup);
            let found = service.listings_by_symbol(&symbol)?;
            Ok::<_, UiError>(service.probe_listings(found, PROBE_LIMIT))
        })
        .await
        .map_err(|e| UiError::internal(format!("the venue lookup task did not run: {e}")))?;
    };

    if !refresh {
        let cached = state.store()?.listings(&isin)?;
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    let isin_for_task = isin.clone();
    let probed = tauri::async_runtime::spawn_blocking(move || {
        let service = sq_core::sources::quote_service_with(&setup);
        let found = service.listings(&isin_for_task)?;
        Ok::<_, UiError>(service.probe_listings(found, PROBE_LIMIT))
    })
    .await
    .map_err(|e| UiError::internal(format!("the venue lookup task did not run: {e}")))??;

    // Written through the main connection once the network is done — the lock is never held
    // across the lookup, and no second connection is opened for three rows.
    let store = state.store()?;
    store.save_listings(&isin, &probed)?;
    for listing in &probed {
        store.update_listing_probe(listing)?;
    }
    Ok(probed)
}

#[derive(Debug, Deserialize)]
pub struct ListingChoice {
    pub security_id: String,
    pub symbol: String,
    pub currency: String,
    pub mic: Option<String>,
}

#[tauri::command]
pub fn security_set_listing(
    app: AppHandle,
    state: State<AppState>,
    choice: ListingChoice,
) -> UiResult<Security> {
    let symbol = choice.symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err(UiError::invalid("the listing symbol is empty"));
    }
    let currency = crate::commands::portfolio::require_currency(&choice.currency)?;

    // Taken before the store's lock: naming a venue for an instrument that has no source yet
    // gives it the first one the owner switched on, and none at all while they have not.
    let default_source = sq_core::sources::default_quotes(&state.market_setup());
    let store = state.store()?;
    let existing = store.get_security(&choice.security_id)?;
    let same_series = existing.provider_symbol() == symbol;
    let updated = Security {
        symbol,
        currency,
        data_source: existing
            .data_source
            .clone()
            .or(default_source.map(str::to_string)),
        data_symbol: None,
        mic: choice
            .mic
            .map(|m| m.trim().to_uppercase())
            .filter(|m| !m.is_empty()),
        ..existing
    };
    store.save_security(&updated)?;
    // The quotes belong to the symbol, not to the MIC: naming the venue of the series already
    // stored is a correction, and throwing it away would refetch what we have.
    if !same_series {
        store.delete_quotes(&updated.id)?;
    }
    drop(store);
    crate::jobs::fetch_missing(&app, &state);

    emit_changed(&app, "securities")?;
    Ok(updated)
}
