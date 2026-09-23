use crate::commands::portfolio::{require_currency, require_text};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::prelude::*;
use std::str::FromStr;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct SecurityRow {
    #[serde(flatten)]
    pub security: Security,
    pub transaction_count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub effective_quantity_step: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub quantity_step_observed: Option<Decimal>,
    pub needs_lookup: bool,
    pub quote_currency: Option<String>,
    pub venue: Option<String>,
    pub quote_count: usize,
    pub coverage_from: Option<String>,
    pub coverage_to: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub last_close: Option<Decimal>,
    /// Values of the user's own attributes, keyed by attribute id.
    pub attributes: AttributeValues,
    /// The instrument's symbol at sources other than its own (`source -> symbol`): what a
    /// fallback may ask when the own source fails (ADR-0052).
    pub other_symbols: std::collections::BTreeMap<String, String>,
    /// The stored series covers far less than the instrument has been held — what a ticker on a
    /// venue the source does not quote leaves behind. The table says so rather than showing a
    /// price that is one day old and years out of place.
    pub sparse_history: bool,
}

#[tauri::command]
pub fn securities_list(state: State<AppState>) -> UiResult<Vec<SecurityRow>> {
    let store = state.store()?;
    let accounts: Vec<String> = store.list_accounts()?.into_iter().map(|a| a.id).collect();
    let transactions = store.transactions_for_accounts(&accounts, None)?;
    let securities = store.list_securities()?;
    let stats = store.quote_stats()?;
    let mut attributes = store.security_attributes()?;
    let mut other_symbols = store.all_security_symbols()?;
    let today = chrono::Local::now().date_naive();

    Ok(securities
        .into_iter()
        .map(|security| {
            let values = attributes.remove(&security.id).unwrap_or_default();
            let others = other_symbols.remove(&security.id).unwrap_or_default();
            let mine = || {
                transactions
                    .iter()
                    .filter(|t| t.security_id.as_deref() == Some(security.id.as_str()))
            };
            let transaction_count = mine().count();
            let observed = observed_quantity_step(mine().map(|t| t.quantity));
            let held_from = mine().map(|t| t.date).min();
            let stat = stats.get(&security.id);
            SecurityRow {
                sparse_history: security.data_source.is_some()
                    && crate::jobs::sparse_history(held_from, stat.map(|q| q.range), today),
                effective_quantity_step: security.quantity_step_for(observed),
                quantity_step_observed: observed,
                needs_lookup: security.needs_lookup(),
                quote_currency: stat.map(|q| q.currency.clone()),
                quote_count: stat.map(|q| q.count).unwrap_or(0),
                last_close: stat.map(|q| q.last_close),
                coverage_from: stat.map(|q| q.range.from.to_string()),
                coverage_to: stat.map(|q| q.range.to.to_string()),
                venue: security
                    .mic
                    .as_deref()
                    .and_then(sq_core::market::mic::market_name)
                    .map(str::to_string),
                transaction_count,
                attributes: values,
                other_symbols: others,
                security,
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
pub struct SecurityInput {
    pub id: Option<String>,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    pub kind: SecurityKind,
    pub isin: Option<String>,
    /// Absent keeps the venue the instrument already carries; only the picker clears one.
    pub mic: Option<String>,
    pub data_source: Option<String>,
    pub data_symbol: Option<String>,
    pub quantity_step: Option<String>,
    pub wkn: Option<String>,
    pub note: Option<String>,
    /// Every attribute the form offered; a blank value clears that one. Absent means the caller
    /// is not editing attributes at all — an empty map would clear every one of them.
    pub attributes: Option<AttributeValues>,
    /// Symbols at other sources, `source -> symbol`; a blank one forgets that source. Absent means
    /// the caller is not editing them, as with `attributes`.
    #[serde(default)]
    pub other_symbols: Option<std::collections::BTreeMap<String, String>>,
}

#[tauri::command]
pub fn security_save(app: AppHandle, state: State<AppState>, input: SecurityInput) -> UiResult<Security> {
    let symbol = require_text(&input.symbol, "ticker")?.to_uppercase();
    let name = require_text(&input.name, "name")?;
    let currency = require_currency(&input.currency)?;
    let quantity_step = parse_step(input.quantity_step.as_deref())?;

    let security = {
        let store = state.store()?;
        let base = match &input.id {
            Some(id) => store.get_security(id)?,
            None => Security::new(&symbol, &name, &currency, input.kind),
        };
        let security = Security {
            symbol,
            name,
            currency,
            kind: input.kind,
            isin: blank_to_none(input.isin),
            mic: blank_to_none(input.mic)
                .map(|m| m.to_uppercase())
                .or(base.mic.clone()),
            data_source: blank_to_none(input.data_source),
            data_symbol: blank_to_none(input.data_symbol),
            quantity_step,
            wkn: blank_to_none(input.wkn),
            note: blank_to_none(input.note),
            ..base
        };
        store.save_security(&security)?;
        if let Some(values) = &input.attributes {
            store.set_security_attributes(&security.id, values)?;
        }
        for (source, symbol) in input.other_symbols.iter().flatten() {
            // The own source reads `data_symbol`; a row for it here would never be asked.
            let own = security.data_source.as_deref() == Some(source.as_str());
            store.set_security_symbol(&security.id, source, if own { "" } else { symbol })?;
        }
        security
    };

    crate::jobs::fetch_missing(&app, &state);
    emit_changed(&app, "securities")?;
    Ok(security)
}

#[tauri::command]
pub async fn security_identify(app: AppHandle, state: State<'_, AppState>, id: String) -> UiResult<Security> {
    let existing = state.store()?.get_security(&id)?;

    let queries: Vec<String> = [
        existing.isin.clone(),
        Some(existing.symbol.clone()),
        Some(existing.name.clone()).filter(|n| n.trim().len() > 2 && *n != existing.symbol),
    ]
    .into_iter()
    .flatten()
    .collect();

    let preferred = existing.currency.clone();
    let setup = state.market_setup();
    let found = tauri::async_runtime::spawn_blocking(move || {
        let service = sq_core::sources::quote_service_with(&setup);
        for query in queries {
            if let Some(found) = service.resolve_preferring(&query, Some(&preferred))? {
                return Ok::<_, UiError>(Some(found));
            }
        }
        Ok(None)
    })
    .await
    .map_err(|e| UiError::internal(format!("the lookup task did not run: {e}")))??;

    let found = found.ok_or_else(|| {
        UiError::invalid(format!(
            "{} was not found in the directory — set the provider ticker by hand",
            existing.symbol
        ))
    })?;

    let updated = Security {
        symbol: found.symbol.to_uppercase(),
        name: found.name,
        currency: found.currency.unwrap_or(existing.currency.clone()),
        kind: found.kind,
        isin: existing.isin.clone().or(found.isin),
        data_source: Some(found.source),
        data_symbol: None,
        // The source names the venue it quotes; a resolution that does not know it clears the old
        // one, which belonged to the symbol being replaced.
        mic: found.mic,
        ..existing
    };
    state.store()?.save_security(&updated)?;
    crate::jobs::fetch_missing(&app, &state);
    emit_changed(&app, "securities")?;
    Ok(updated)
}

#[tauri::command]
pub fn security_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_security(&id)?;
    emit_changed(&app, "securities")
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn parse_step(value: Option<&str>) -> UiResult<Option<Decimal>> {
    let Some(raw) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let step = Decimal::from_str(raw)
        .map_err(|e| UiError::invalid(format!("invalid quantity step {raw:?}: {e}")))?;
    if step <= Decimal::ZERO {
        return Err(UiError::invalid("the quantity step must be greater than zero"));
    }
    Ok(Some(step))
}

#[tauri::command]
pub fn quotes_range(
    state: State<AppState>,
    security_id: String,
    from: String,
    to: String,
) -> UiResult<Vec<Quote>> {
    let range = DateRange::new(
        crate::commands::parse_date(&from)?,
        crate::commands::parse_date(&to)?,
    );
    let store = state.store()?;
    Ok(store.quotes_in_range(&security_id, range)?)
}
