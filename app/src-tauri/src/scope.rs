//! Active data scope used by all analytics commands.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::prelude::*;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScopeKind {
    /// The complete portfolio.
    #[default]
    Portfolio,
    /// A saved account group.
    Group,
    /// One account.
    Account,
    /// A securities account together with its settlement account.
    AccountWithCash,
}

/// Current scope selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DataScope {
    pub kind: ScopeKind,
    pub id: Option<String>,
}

impl DataScope {
    pub fn portfolio() -> Self {
        DataScope::default()
    }

    /// Apply the selection while excluding accounts outside the portfolio.
    pub fn apply(&self, portfolio: &Portfolio, groups: &[AccountGroup], accounts: &[Account]) -> Portfolio {
        let ids: Option<Vec<String>> = match (self.kind, self.id.as_deref()) {
            (ScopeKind::Portfolio, _) | (_, None) => None,
            (ScopeKind::Group, Some(id)) => groups.iter().find(|g| g.id == id).map(|g| {
                portfolio
                    .account_ids
                    .iter()
                    .filter(|acc| g.account_ids.contains(acc))
                    .cloned()
                    .collect()
            }),
            (ScopeKind::Account, Some(id)) => portfolio
                .account_ids
                .iter()
                .find(|acc| acc.as_str() == id)
                .map(|acc| vec![acc.clone()]),
            (ScopeKind::AccountWithCash, Some(id)) => accounts.iter().find(|a| a.id == id).map(|account| {
                let pair = [account.id.as_str(), account.settlement_account_id()];
                portfolio
                    .account_ids
                    .iter()
                    .filter(|acc| pair.contains(&acc.as_str()))
                    .cloned()
                    .collect()
            }),
        };

        match ids {
            Some(account_ids) => Portfolio {
                account_ids,
                ..portfolio.clone()
            },
            None => portfolio.clone(),
        }
    }
}

/// One navigation scope option. It carries names, not a sentence: the wording of
/// "Whole portfolio · Main" belongs to the frontend, which is where the language lives.
#[derive(Debug, Serialize)]
pub struct ScopeOption {
    pub kind: ScopeKind,
    pub id: Option<String>,
    /// Portfolio, group or account name.
    pub name: String,
    /// Kind of the account this option stands for; absent for a portfolio or a group.
    pub account_kind: Option<AccountKind>,
    /// Name of the settlement account, set only for `AccountWithCash`.
    pub cash_name: Option<String>,
    /// Number of accounts included by this option.
    pub account_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ScopeState {
    pub scope: DataScope,
    pub options: Vec<ScopeOption>,
}

#[tauri::command]
pub fn scope_get(state: State<AppState>) -> UiResult<ScopeState> {
    let store = state.store()?;
    let portfolio = state.portfolio()?;
    let groups = store.list_account_groups()?;
    let accounts = store.list_accounts()?;
    let scope = state.scope()?.clone();

    let mut options = vec![ScopeOption {
        kind: ScopeKind::Portfolio,
        id: None,
        name: portfolio.name.clone(),
        account_kind: None,
        cash_name: None,
        account_count: portfolio.account_ids.len(),
    }];
    for group in &groups {
        options.push(ScopeOption {
            kind: ScopeKind::Group,
            id: Some(group.id.clone()),
            name: group.name.clone(),
            account_kind: None,
            cash_name: None,
            account_count: scope_count(portfolio.account_ids.as_slice(), &group.account_ids),
        });
    }
    for account in &accounts {
        if !portfolio.account_ids.contains(&account.id) {
            continue;
        }
        options.push(ScopeOption {
            kind: ScopeKind::Account,
            id: Some(account.id.clone()),
            name: account.name.clone(),
            account_kind: Some(account.kind),
            cash_name: None,
            account_count: 1,
        });

        if account.kind == AccountKind::Securities {
            let cash = account.settlement_account_id();
            if let Some(cash) = accounts
                .iter()
                .find(|a| a.id == cash && portfolio.account_ids.contains(&a.id))
            {
                options.push(ScopeOption {
                    kind: ScopeKind::AccountWithCash,
                    id: Some(account.id.clone()),
                    name: account.name.clone(),
                    account_kind: Some(account.kind),
                    cash_name: Some(cash.name.clone()),
                    account_count: 2,
                });
            }
        }
    }

    Ok(ScopeState { scope, options })
}

fn scope_count(portfolio: &[String], group: &[String]) -> usize {
    group.iter().filter(|id| portfolio.contains(id)).count()
}

/// Change the active scope and invalidate dependent queries.
#[tauri::command]
pub fn scope_set(app: AppHandle, state: State<AppState>, scope: DataScope) -> UiResult<DataScope> {
    {
        let store = state.store()?;
        match (scope.kind, scope.id.as_deref()) {
            (ScopeKind::Group, Some(id)) => {
                store.get_account_group(id)?;
            }
            (ScopeKind::Account | ScopeKind::AccountWithCash, Some(id)) => {
                store.get_account(id)?;
            }
            (ScopeKind::Portfolio, _) => {}
            (_, None) => return Err(UiError::invalid("no account or group selected")),
        }
    }

    *state.scope()? = scope.clone();
    state.persist_settings()?;
    crate::events::emit_changed(&app, "scope")?;
    Ok(scope)
}
