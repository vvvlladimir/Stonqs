use super::off_thread;
use crate::error::UiResult;
use crate::state::AppState;
use sq_core::import::SecurityDraft;
use sq_core::market::SecurityMatch;
use sq_core::model::is_isin;
use sq_core::sources::quote_service_with;
use tauri::State;

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

        // No venue from the search: ask the directory by ISIN, else by the bare ticker.
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
