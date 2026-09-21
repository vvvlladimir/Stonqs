//! Which market-data sources take part, and their keys. The catalogue itself is the core's
//! (`sq_core::sources`); the host only adds what the user decided. See ADR-0053.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use chrono::{Duration, Utc};
use serde::Serialize;
use sq_core::fx::FxProvider;
use sq_core::market::{CUSTOM_PREFIX, CustomProvider, CustomRole, CustomSource, DateRange, QuoteProvider};
use sq_core::model::{Security, SecurityKind};
use sq_core::sources::{self, Capability, KeyUse, Setup};
use tauri::State;

/// The vault account a source's key is kept under, apart from the AI providers' own.
fn account(source: &str) -> String {
    format!("market:{source}")
}

impl AppState {
    /// The user's switches and saved keys, copied out so a background job holds no lock for them.
    /// A locked profile yields no keys: its keyed sources simply stay off until it is unlocked.
    pub fn market_setup(&self) -> Setup {
        let mut setup = Setup::default();
        for source in sources::CATALOG.iter().filter(|s| s.key != KeyUse::None) {
            if let Ok(key) = self.key_for_call(&account(source.id))
                && !key.is_empty()
            {
                setup.keys.insert(source.id.to_string(), key);
            }
        }
        if let Ok(settings) = self.settings() {
            setup.switched = settings.market_sources.clone().into_iter().collect();
            setup.custom = settings.market_custom.clone();
        }
        for custom in &setup.custom {
            if let Ok(key) = self.key_for_call(&account(&custom.id))
                && !key.is_empty()
            {
                setup.keys.insert(custom.id.clone(), key);
            }
        }
        setup
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketSourceRow {
    pub id: &'static str,
    /// `quotes | search | listings | fx_rates`.
    pub capabilities: Vec<&'static str>,
    /// `none | optional | required`.
    pub key: &'static str,
    pub has_key: bool,
    pub on_by_default: bool,
    /// Switched on by the user or by default, whether or not a missing key keeps it out.
    pub wanted: bool,
    /// Actually asked: wanted and not missing a required key.
    pub active: bool,
}

#[tauri::command]
pub fn market_sources_list(state: State<AppState>) -> UiResult<Vec<MarketSourceRow>> {
    let setup = state.market_setup();
    Ok(sources::CATALOG
        .iter()
        .map(|s| MarketSourceRow {
            id: s.id,
            capabilities: s
                .capabilities()
                .into_iter()
                .map(|c| match c {
                    Capability::Quotes => "quotes",
                    Capability::Search => "search",
                    Capability::Listings => "listings",
                    Capability::FxRates => "fx_rates",
                    Capability::PriceIndex => "price_index",
                })
                .collect(),
            key: match s.key {
                KeyUse::None => "none",
                KeyUse::Optional => "optional",
                KeyUse::Required => "required",
            },
            has_key: setup.keys.contains_key(s.id),
            on_by_default: s.on_by_default,
            wanted: setup.switched.get(s.id).copied().unwrap_or(s.on_by_default),
            active: setup.is_on(s),
        })
        .collect())
}

/// Switches a source on or off; a switch back to the default is forgotten rather than stored.
#[tauri::command]
pub fn market_source_switch(state: State<AppState>, source: String, on: bool) -> UiResult<()> {
    let default = match sources::info(&source) {
        Some(row) => row.on_by_default,
        None if source.starts_with(CUSTOM_PREFIX) => true,
        None => return Err(UiError::invalid(format!("unknown source {source}"))),
    };
    {
        let mut settings = state.settings()?;
        if on == default {
            settings.market_sources.remove(&source);
        } else {
            settings.market_sources.insert(source, on);
        }
    }
    state.persist_settings()
}

#[tauri::command]
pub fn market_key_save(state: State<AppState>, source: String, key: String) -> UiResult<()> {
    if !source.starts_with(CUSTOM_PREFIX) {
        let row =
            sources::info(&source).ok_or_else(|| UiError::invalid(format!("unknown source {source}")))?;
        if row.key == KeyUse::None {
            return Err(UiError::invalid(format!("{source} takes no key")));
        }
    }
    state.key_save(&account(&source), key.trim())
}

#[tauri::command]
pub fn market_key_delete(state: State<AppState>, source: String) -> UiResult<()> {
    state.key_delete(&account(&source))
}

fn checked(def: &CustomSource) -> UiResult<()> {
    def.validate()
        .map_err(|code| UiError::invalid(format!("custom source {}: {code}", def.id)))
}

#[tauri::command]
pub fn market_custom_list(state: State<AppState>) -> UiResult<Vec<CustomSource>> {
    Ok(state.settings()?.market_custom.clone())
}

/// Adds the definition, or replaces the one with the same id.
#[tauri::command]
pub fn market_custom_save(state: State<AppState>, source: CustomSource) -> UiResult<()> {
    checked(&source)?;
    {
        let mut settings = state.settings()?;
        match settings.market_custom.iter_mut().find(|c| c.id == source.id) {
            Some(slot) => *slot = source,
            None => settings.market_custom.push(source),
        }
    }
    state.persist_settings()
}

/// Removes the definition and its key. Instruments that name it keep the id and show it unknown
/// until they are pointed elsewhere — their quotes are not deleted with a source definition.
#[tauri::command]
pub fn market_custom_delete(state: State<AppState>, id: String) -> UiResult<()> {
    {
        let mut settings = state.settings()?;
        settings.market_custom.retain(|c| c.id != id);
        settings.market_sources.remove(&id);
    }
    state.persist_settings()?;
    state.key_delete(&account(&id))
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomTestRow {
    pub date: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub close: rust_decimal::Decimal,
    pub currency: String,
}

/// Asks an unsaved definition for the last 30 days of `symbol` and returns the newest ten rows,
/// so the form can show what the paths actually read before anything is stored. A rate source
/// reads `symbol` as a pair, `EUR/USD`; each row's `currency` is then the quote currency.
#[tauri::command]
pub async fn market_custom_test(
    state: State<'_, AppState>,
    source: CustomSource,
    symbol: String,
    currency: String,
) -> UiResult<Vec<CustomTestRow>> {
    checked(&source)?;
    let key = state
        .key_for_call(&account(&source.id))
        .ok()
        .filter(|k| !k.is_empty());
    tauri::async_runtime::spawn_blocking(move || {
        let provider = CustomProvider::new(source.clone(), key);
        let today = Utc::now().date_naive();
        let range = DateRange::new(today - Duration::days(30), today);
        let mut quotes: Vec<(chrono::NaiveDate, rust_decimal::Decimal, String)> = match source.role {
            CustomRole::Quotes => {
                let probe = Security::new(&symbol, &symbol, &currency, SecurityKind::Other)
                    .with_source(&source.id, &symbol);
                QuoteProvider::fetch(&provider, &probe, range)?
                    .into_iter()
                    .map(|q| (q.date, q.close, q.currency))
                    .collect()
            }
            CustomRole::Fx => {
                let pair = symbol.replace(['/', '-', ' '], "");
                if pair.len() != 6 {
                    return Err(UiError::invalid(format!(
                        "{symbol} is not a currency pair like EUR/USD"
                    )));
                }
                let (base, quote) = pair.split_at(3);
                FxProvider::fetch(&provider, base, quote, range)?
                    .into_iter()
                    .map(|r| (r.date, r.rate, r.quote))
                    .collect()
            }
        };
        quotes.reverse();
        quotes.truncate(10);
        Ok::<_, UiError>(
            quotes
                .into_iter()
                .map(|(date, close, currency)| CustomTestRow {
                    date: date.to_string(),
                    close,
                    currency,
                })
                .collect(),
        )
    })
    .await
    .map_err(|e| UiError::internal(format!("the test task did not run: {e}")))?
}
