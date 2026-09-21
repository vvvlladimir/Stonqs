use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use serde::Deserialize;
use sq_core::prelude::*;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn portfolio_get(state: State<AppState>) -> UiResult<Portfolio> {
    Ok(state.portfolio()?.clone())
}

#[derive(Debug, Deserialize)]
pub struct PortfolioInput {
    pub name: String,
    pub base_currency: String,
    pub cost_basis_method: CostBasisMethod,
    pub account_ids: Vec<String>,
}

#[tauri::command]
pub fn portfolio_save(app: AppHandle, state: State<AppState>, input: PortfolioInput) -> UiResult<Portfolio> {
    let name = require_text(&input.name, "portfolio name")?;
    let base_currency = require_currency(&input.base_currency)?;

    let updated = {
        let store = state.store()?;
        let mut portfolio = state.portfolio()?.clone();
        portfolio.name = name;
        portfolio.base_currency = base_currency;
        portfolio.cost_basis_method = input.cost_basis_method;
        portfolio.account_ids = input.account_ids;
        store.save_portfolio(&portfolio)?;
        portfolio
    };

    state.reload_portfolio()?;
    emit_changed(&app, "portfolio")?;
    Ok(updated)
}

#[derive(Debug, Deserialize)]
pub struct SetupInput {
    pub portfolio_name: String,
    pub base_currency: String,
    pub account_name: String,
    pub account_currency: String,
    #[serde(default)]
    pub securities_account_name: Option<String>,
}

#[tauri::command]
pub fn setup_portfolio(app: AppHandle, state: State<AppState>, input: SetupInput) -> UiResult<Portfolio> {
    let portfolio_name = require_text(&input.portfolio_name, "portfolio name")?;
    let base_currency = require_currency(&input.base_currency)?;
    let account_name = require_text(&input.account_name, "account name")?;
    let account_currency = require_currency(&input.account_currency)?;

    let depot_name = input
        .securities_account_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string);

    let updated = {
        let store = state.store()?;
        let account = Account::deposit(account_name, &account_currency);
        store.save_account(&account)?;

        let mut portfolio = state.portfolio()?.clone();
        portfolio.name = portfolio_name;
        portfolio.base_currency = base_currency;
        if !portfolio.account_ids.contains(&account.id) {
            portfolio.account_ids.push(account.id.clone());
        }
        if let Some(name) = depot_name {
            let depot = Account::securities(name, &account_currency, &account.id);
            store.save_account(&depot)?;
            portfolio.account_ids.push(depot.id.clone());
        }
        store.save_portfolio(&portfolio)?;
        portfolio
    };

    state.reload_portfolio()?;
    emit_changed(&app, "portfolio")?;
    Ok(updated)
}

pub(crate) fn require_text(value: &str, what: &str) -> UiResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(UiError::invalid(format!("missing: {what}")));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn require_currency(value: &str) -> UiResult<String> {
    let code = sq_core::money::normalize_currency(value);
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(UiError::invalid(format!(
            "a currency code must be three letters; got {value:?}"
        )));
    }
    Ok(code)
}
