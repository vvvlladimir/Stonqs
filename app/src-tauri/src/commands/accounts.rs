//! Cash accounts, securities accounts, and account groups.

use crate::commands::portfolio::{require_currency, require_text};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::cash_balances;
use sq_core::prelude::*;
use tauri::{AppHandle, State};

/// A balance for one currency held by an account.
#[derive(Debug, Serialize)]
pub struct CashBalance {
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

/// Account data plus application-specific display fields.
#[derive(Debug, Serialize)]
pub struct AccountRow {
    #[serde(flatten)]
    pub account: Account,
    pub transaction_count: usize,
    /// Whether the account belongs to the current portfolio.
    pub in_portfolio: bool,
    /// Cash account used to settle a securities account.
    pub reference_name: Option<String>,
    /// Cash balances; securities accounts always return an empty list.
    pub balances: Vec<CashBalance>,
    /// Securities value in the portfolio currency, or `None` if data is missing.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub securities_value_base: Option<Decimal>,
    /// Account value in the portfolio currency, calculated by the core.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub value_base: Option<Decimal>,
}

#[tauri::command]
pub fn accounts_list(state: State<AppState>) -> UiResult<Vec<AccountRow>> {
    let store = state.store()?;
    let portfolio = state.portfolio()?;
    let today = chrono::Local::now().date_naive();

    let accounts = store.list_accounts()?;
    let ids: Vec<String> = accounts.iter().map(|a| a.id.clone()).collect();
    let transactions = store.transactions_for_accounts(&ids, None)?;
    let balances = cash_balances(&transactions, &accounts, today);

    accounts
        .iter()
        .map(|account| {
            let transaction_count = transactions.iter().filter(|t| t.account_id == account.id).count();
            let reference_name = account
                .reference_account_id
                .as_ref()
                .and_then(|id| accounts.iter().find(|a| &a.id == id))
                .map(|a| a.name.clone());
            let valuation = value_of(&store, &portfolio, std::slice::from_ref(&account.id), today);
            let securities_value_base = match account.kind {
                AccountKind::Securities => valuation.as_ref().map(|v| v.securities_value_base),
                AccountKind::Deposit => None,
            };

            Ok(AccountRow {
                in_portfolio: portfolio.account_ids.contains(&account.id),
                transaction_count,
                reference_name,
                balances: balances
                    .get(&account.id)
                    .map(|by_currency| {
                        by_currency
                            .iter()
                            .map(|(currency, amount)| CashBalance {
                                currency: currency.clone(),
                                amount: *amount,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                securities_value_base,
                value_base: valuation.as_ref().map(|v| v.total_value_base),
                account: account.clone(),
            })
        })
        .collect()
}

/// Value an account or group using the core's scoped portfolio rules.
fn value_of(
    store: &Store,
    portfolio: &Portfolio,
    accounts: &[String],
    date: chrono::NaiveDate,
) -> Option<PortfolioValuation> {
    let analytics = PortfolioAnalytics::new(store, portfolio)
        .ok()?
        .scoped_to(accounts);
    analytics.valuation_at(date).ok()
}

#[derive(Debug, Deserialize)]
pub struct AccountInput {
    /// `None` creates a new account.
    pub id: Option<String>,
    pub name: String,
    pub currency: String,
    pub kind: AccountKind,
    /// Settlement account for a securities account; `None` for cash accounts.
    pub reference_account_id: Option<String>,
    pub is_active: bool,
    pub opened_at: Option<String>,
}

/// Create or update an account; new accounts are included in the portfolio.
#[tauri::command]
pub fn account_save(app: AppHandle, state: State<AppState>, input: AccountInput) -> UiResult<Account> {
    let name = require_text(&input.name, "account name")?;
    let currency = require_currency(&input.currency)?;
    let opened_at = input.opened_at.as_deref().map(super::parse_date).transpose()?;
    let reference_account_id = match input.kind {
        AccountKind::Securities => Some(
            input
                .reference_account_id
                .clone()
                .ok_or_else(|| UiError::invalid("a securities account needs a cash account"))?,
        ),
        AccountKind::Deposit => None,
    };

    let account = {
        let store = state.store()?;
        let account = match &input.id {
            Some(id) => {
                let existing = store.get_account(id)?;
                Account {
                    name,
                    currency,
                    kind: input.kind,
                    reference_account_id,
                    is_active: input.is_active,
                    opened_at,
                    ..existing
                }
            }
            None => Account {
                name,
                currency,
                kind: input.kind,
                reference_account_id,
                is_active: input.is_active,
                opened_at,
                ..Account::deposit("", "EUR")
            },
        };
        store.save_account(&account)?;

        if input.id.is_none() {
            let mut portfolio = state.portfolio()?.clone();
            portfolio.account_ids.push(account.id.clone());
            store.save_portfolio(&portfolio)?;
        }
        account
    };

    state.reload_portfolio()?;
    emit_changed(&app, "accounts")?;
    Ok(account)
}

/// Delete an account, requiring confirmation when it has transactions.
#[tauri::command]
pub fn account_delete(app: AppHandle, state: State<AppState>, id: String, force: bool) -> UiResult<()> {
    {
        let store = state.store()?;
        let dependents = store.accounts_referencing(&id)?;
        if !dependents.is_empty() {
            let names: Vec<&str> = dependents.iter().map(|a| a.name.as_str()).collect();
            return Err(UiError::invalid(format!(
                "securities accounts settle through this one: {} — relink them first",
                names.join(", ")
            )));
        }

        let count = store.transactions_for_account(&id)?.len();
        if count > 0 && !force {
            return Err(UiError::invalid(format!(
                "the account carries {count} transactions; they are deleted with it"
            )));
        }
        store.delete_account(&id)?;
    }

    let stale = {
        let scope = state.scope()?;
        scope.kind == crate::scope::ScopeKind::Account && scope.id.as_deref() == Some(id.as_str())
    };
    if stale {
        *state.scope()? = crate::scope::DataScope::portfolio();
        state.persist_settings()?;
    }

    state.reload_portfolio()?;
    emit_changed(&app, "accounts")
}

/// Account group plus fields needed by the list view.
#[derive(Debug, Serialize)]
pub struct AccountGroupRow {
    #[serde(flatten)]
    pub group: AccountGroup,
    /// Member names, loaded with the group to avoid per-group queries.
    pub account_names: Vec<String>,
    /// Scoped value in the portfolio currency, not a sum of account cards.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub value_base: Option<Decimal>,
}

#[tauri::command]
pub fn account_groups_list(state: State<AppState>) -> UiResult<Vec<AccountGroupRow>> {
    let store = state.store()?;
    let portfolio = state.portfolio()?;
    let today = chrono::Local::now().date_naive();
    let accounts = store.list_accounts()?;
    Ok(store
        .list_account_groups()?
        .into_iter()
        .map(|group| AccountGroupRow {
            account_names: group
                .account_ids
                .iter()
                .filter_map(|id| accounts.iter().find(|a| &a.id == id))
                .map(|a| a.name.clone())
                .collect(),
            value_base: value_of(&store, &portfolio, &group.account_ids, today).map(|v| v.total_value_base),
            group,
        })
        .collect())
}

/// Total value for all accounts in the current portfolio.
#[derive(Debug, Serialize)]
pub struct AccountsTotal {
    pub base_currency: String,
    /// `None` means a quote or FX rate is missing.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub value_base: Option<Decimal>,
}

#[tauri::command]
pub fn accounts_total(state: State<AppState>) -> UiResult<AccountsTotal> {
    let store = state.store()?;
    let portfolio = state.portfolio()?;
    let today = chrono::Local::now().date_naive();
    let ids = portfolio.account_ids.clone();
    Ok(AccountsTotal {
        base_currency: portfolio.base_currency.clone(),
        value_base: value_of(&store, &portfolio, &ids, today).map(|v| v.total_value_base),
    })
}

#[derive(Debug, Deserialize)]
pub struct AccountGroupInput {
    pub id: Option<String>,
    pub name: String,
    pub account_ids: Vec<String>,
}

#[tauri::command]
pub fn account_group_save(
    app: AppHandle,
    state: State<AppState>,
    input: AccountGroupInput,
) -> UiResult<AccountGroup> {
    let name = require_text(&input.name, "group name")?;
    if input.account_ids.is_empty() {
        return Err(UiError::invalid("a group needs at least one account"));
    }

    let group = AccountGroup {
        id: input.id.unwrap_or_else(|| AccountGroup::new("").id),
        name,
        account_ids: input.account_ids,
    };
    state.store()?.save_account_group(&group)?;
    emit_changed(&app, "accounts")?;
    Ok(group)
}

#[tauri::command]
pub fn account_group_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_account_group(&id)?;

    let stale = {
        let scope = state.scope()?;
        scope.kind == crate::scope::ScopeKind::Group && scope.id.as_deref() == Some(id.as_str())
    };
    if stale {
        *state.scope()? = crate::scope::DataScope::portfolio();
        state.persist_settings()?;
    }

    emit_changed(&app, "accounts")
}
