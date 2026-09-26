//! The demo portfolio offered to a new profile.

use crate::demo;
use crate::error::{UiError, UiResult};
use crate::state::AppState;
use chrono::Local;
use tauri::State;

/// Fills an empty portfolio with three years of generated history.
///
/// It refuses once the portfolio has an account, so it can never write over real data. The data
/// itself is `demo`: prices, operations, classifications, goals, plans and alerts.
#[tauri::command]
pub fn demo_seed(state: State<AppState>) -> UiResult<()> {
    {
        let store = state.store()?;
        let mut portfolio = state.portfolio()?;
        if !portfolio.account_ids.is_empty() {
            return Err(UiError::invalid("the portfolio is no longer empty"));
        }
        demo::seed(&store, &mut portfolio, Local::now().date_naive())?;
    }
    state.reload_portfolio()
}
