use crate::commands::named;
use crate::commands::parse_date;
use crate::error::UiResult;
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::PortfolioValuation;
use sq_core::prelude::*;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub portfolio_name: String,
    pub base_currency: String,
    pub inception: Option<String>,
    pub account_count: usize,
    pub db_path: String,
    pub dev_build: bool,
    /// OS language as a BCP-47 tag; the UI resolves it to a locale it ships.
    pub system_locale: Option<String>,
}

#[tauri::command]
pub fn app_status(state: State<AppState>) -> UiResult<AppStatus> {
    let store = state.store()?;
    let scope = state.scope_selection(&store)?;
    let analytics = scope.analytics(&store)?;

    Ok(AppStatus {
        portfolio_name: scope.portfolio.name.clone(),
        base_currency: scope.portfolio.base_currency.clone(),
        inception: analytics.inception()?.map(|d| d.to_string()),
        account_count: store.list_accounts()?.len(),
        db_path: state.db_path()?.display().to_string(),
        dev_build: cfg!(debug_assertions),
        system_locale: sys_locale::get_locale(),
    })
}

#[derive(Debug, Serialize)]
pub struct SecurityRef {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct AccountRef {
    pub id: String,
    pub name: String,
    pub kind: AccountKind,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct CashBalance {
    pub account_id: String,
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub valuation: PortfolioValuation,
    pub securities: Vec<SecurityRef>,
    pub accounts: Vec<AccountRef>,
    pub cash: Vec<CashBalance>,
}

#[tauri::command]
pub fn dashboard_summary(
    state: State<AppState>,
    date: String,
    source: Option<DataScope>,
) -> UiResult<DashboardData> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let valuation = analytics.valuation_at(date).map_err(|e| named(&store, e))?;

    let accounts = analytics
        .scope_accounts()
        .into_iter()
        .map(|a| AccountRef {
            id: a.id.clone(),
            name: a.name.clone(),
            kind: a.kind,
            currency: a.currency.clone(),
        })
        .collect();
    let cash = analytics
        .cash_balances(date)?
        .into_iter()
        .flat_map(|(account_id, by_currency)| {
            by_currency
                .into_iter()
                .map(move |(currency, amount)| CashBalance {
                    account_id: account_id.clone(),
                    currency,
                    amount,
                })
        })
        .collect();

    let securities = store
        .list_securities()?
        .into_iter()
        .map(|s| SecurityRef {
            id: s.id,
            symbol: s.symbol,
            name: s.name,
            currency: s.currency,
        })
        .collect();

    Ok(DashboardData {
        valuation,
        securities,
        accounts,
        cash,
    })
}
