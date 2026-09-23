use crate::error::{UiError, UiResult};
use crate::state::AppState;
use sq_core::import::SecurityDraft;
use sq_core::market::SecurityMatch;
use sq_core::model::is_isin;
use sq_core::sources::quote_service_with;
use tauri::State;

async fn off_thread<T, F>(work: F) -> UiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> UiResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| UiError::internal(format!("the lookup task did not run: {e}")))?
}

#[tauri::command]
pub async fn security_search(state: State<'_, AppState>, query: String) -> UiResult<Vec<SecurityMatch>> {
    let setup = state.market_setup();
    off_thread(move || Ok(quote_service_with(&setup).search(&query)?)).await
}

/// One picked search result's own profile: a search result carries no currency, and resolving
/// its symbol again may land on another listing.
#[tauri::command]
pub async fn security_profile(
    state: State<'_, AppState>,
    source: String,
    symbol: String,
) -> UiResult<Option<SecurityMatch>> {
    let setup = state.market_setup();
    off_thread(move || Ok(quote_service_with(&setup).profile(&source, &symbol)?)).await
}

#[tauri::command]
pub async fn security_resolve(
    state: State<'_, AppState>,
    query: String,
    currency: Option<String>,
) -> UiResult<Option<SecurityMatch>> {
    let setup = state.market_setup();
    off_thread(move || Ok(quote_service_with(&setup).resolve_preferring(&query, currency.as_deref())?)).await
}

#[tauri::command]
pub async fn import_resolve_symbol(
    state: State<'_, AppState>,
    value: String,
    isin: Option<String>,
    name: Option<String>,
    currency: Option<String>,
) -> UiResult<Option<SecurityDraft>> {
    let setup = state.market_setup();
    off_thread(move || {
        let service = quote_service_with(&setup);
        let mut queries: Vec<String> = Vec::new();
        if let Some(isin) = isin.filter(|i| is_isin(i)) {
            queries.push(isin);
        }
        if !queries.iter().any(|q| q.eq_ignore_ascii_case(&value)) {
            queries.push(value.clone());
        }
        if let Some(name) = name.filter(|n| n.trim().len() > 2) {
            queries.push(name);
        }

        let fallback = currency.unwrap_or_else(|| "EUR".to_string());
        for query in &queries {
            if let Some(found) = service.resolve_preferring(query, Some(&fallback))? {
                let mut draft = SecurityDraft::from_match(&found, &fallback);
                if draft.isin.is_none() && is_isin(&value) {
                    draft.isin = Some(value.to_uppercase());
                }
                return Ok(Some(draft));
            }
        }

        // The search places a code on no venue at all when the file names one this source does
        // not index. The directory still can: it is keyed by ISIN, and every venue it returns is
        // probed for candles before one is taken. Without an ISIN the bare ticker is asked
        // instead — a broker's code with a suffix this source spells differently.
        let venue = match queries.iter().find(|q| is_isin(q)) {
            Some(isin) => service.best_listing(isin, Some(&fallback))?,
            None => service.best_listing_by_symbol(&value, Some(&fallback))?,
        };
        let Some(venue) = venue else { return Ok(None) };
        let mut draft = match SecurityDraft::from_listing(&venue, &fallback) {
            Some(draft) => draft,
            None => return Ok(None),
        };
        if draft.isin.is_none() {
            draft.isin = queries.iter().find(|q| is_isin(q)).map(|i| i.to_uppercase());
        }
        Ok(Some(draft))
    })
    .await
}
