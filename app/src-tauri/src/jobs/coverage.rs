//! Per-instrument quote coverage for the data-sources screen.

use crate::error::UiResult;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct DataCoverage {
    pub security_id: String,
    pub symbol: String,
    pub data_source: Option<String>,
    pub last_quote: Option<String>,
    pub first_quote: Option<String>,
    pub quote_count: usize,
    /// Calendar days since the last quote; `null` when there is none.
    pub stale_days: Option<i64>,
    /// Currency of the latest quote.
    pub quote_currency: Option<String>,
    pub mic: Option<String>,
    /// Derived from the MIC, never stored.
    pub venue: Option<String>,
}

#[tauri::command]
pub fn data_coverage(state: State<AppState>) -> UiResult<Vec<DataCoverage>> {
    let store = state.store()?;
    let stats = store.quote_stats()?;
    let today = chrono::Local::now().date_naive();

    Ok(store
        .list_securities()?
        .into_iter()
        .map(|s| {
            let stat = stats.get(&s.id);
            DataCoverage {
                last_quote: stat.map(|q| q.range.to.to_string()),
                first_quote: stat.map(|q| q.range.from.to_string()),
                quote_count: stat.map(|q| q.count).unwrap_or(0),
                stale_days: stat.map(|q| (today - q.range.to).num_days()),
                quote_currency: stat.map(|q| q.currency.clone()),
                venue: s
                    .mic
                    .as_deref()
                    .and_then(sq_core::market::mic::market_name)
                    .map(str::to_string),
                mic: s.mic,
                security_id: s.id,
                symbol: s.symbol,
                data_source: s.data_source,
            }
        })
        .collect())
}
