use crate::commands::named;
use crate::commands::performance::date_range;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::scope::DataScope;
use crate::state::AppState;
use serde::Serialize;
use sq_core::calc::{GrowthSeries, RealPerformance};
use sq_core::inflation::{IndexLookup, regions};
use sq_core::model::normalize_region;
use tauri::{AppHandle, State};

/// What the app knows about the portfolio's price index. Codes only: the region's name is
/// written in the frontend, where the language is known.
#[derive(Debug, Serialize)]
pub struct InflationStatus {
    /// The region the portfolio reports against; `None` when real returns are switched off.
    pub region: Option<String>,
    /// Which source answered it, `None` before the first fetch.
    pub source: Option<String>,
    /// The last month a level is stored for, as its first day.
    pub published_through: Option<String>,
    /// Every region this build can fetch, for the picker.
    pub regions: Vec<&'static str>,
}

#[tauri::command]
pub fn inflation_status(state: State<AppState>) -> UiResult<InflationStatus> {
    let region = state.portfolio()?.inflation_region.clone();
    let store = state.store()?;
    let (source, published_through) = match region.as_deref() {
        Some(region) => (
            store.index_coverage(region)?.map(|(_, source)| source),
            store.index_through(region)?.map(|m| m.to_string()),
        ),
        None => (None, None),
    };
    Ok(InflationStatus {
        region,
        source,
        published_through,
        regions: regions(),
    })
}

/// Points the portfolio at a consumer-price region, or switches real returns off with `None`.
/// The index of a region nobody has asked for yet is fetched in the background.
#[tauri::command]
pub fn inflation_region_set(app: AppHandle, state: State<AppState>, region: Option<String>) -> UiResult<()> {
    let region = match region {
        Some(region) => {
            let region = normalize_region(&region);
            if !regions().contains(&region.as_str()) {
                return Err(UiError::invalid("no source publishes an index for that region"));
            }
            Some(region)
        }
        None => None,
    };
    {
        let store = state.store()?;
        let mut portfolio = state.portfolio()?.clone();
        portfolio.inflation_region = region;
        store.save_portfolio(&portfolio)?;
    }
    state.reload_portfolio()?;
    emit_changed(&app, "portfolio")?;
    crate::jobs::fetch_missing(&app, &state);
    Ok(())
}

/// The period's returns with inflation taken out; `None` when no region is set.
#[tauri::command]
pub fn real_performance(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<Option<RealPerformance>> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .real_performance(range.from, range.to)
        .map_err(|e| named(&store, e))
}

/// What one unit of money costs across the period, based at its first day — the shape a
/// benchmark has, so the chart draws it as one more line. `None` when no region is set.
#[tauri::command]
pub fn inflation_series(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<Option<GrowthSeries>> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let dates: Vec<chrono::NaiveDate> = std::iter::successors(Some(range.from), |d| {
        d.succ_opt().filter(|next| *next <= range.to)
    })
    .collect();
    scope
        .analytics(&store)?
        .inflation_growth(&dates)
        .map_err(|e| named(&store, e))
}
