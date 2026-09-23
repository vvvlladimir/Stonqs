//! Savings goals and contribution limits. Neither is scoped: a goal carries its own accounts and
//! a limit belongs to one account, so the picker changes neither (ADR-0068).

use crate::commands::alerts::today;
use crate::commands::parse_date;
use crate::commands::portfolio::require_text;
use crate::commands::transactions::decimal;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::{GoalProgress, LimitUsage};
use sq_core::prelude::*;
use tauri::{AppHandle, State};

/// A goal with the names its ids stand for, so the list reads without a join.
#[derive(Debug, Serialize)]
pub struct GoalRow {
    pub goal: Goal,
    pub account_names: Vec<String>,
    pub progress: GoalProgress,
}

#[derive(Debug, Deserialize)]
pub struct GoalInput {
    pub id: Option<String>,
    pub name: String,
    pub target_amount: String,
    pub currency: String,
    pub target_date: Option<String>,
    pub monthly_amount: Option<String>,
    /// Percent, as the user typed it: `5` is five percent a year.
    pub expected_return: Option<String>,
    pub note: Option<String>,
    pub accounts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitInput {
    pub id: Option<String>,
    pub account_id: String,
    pub name: String,
    pub amount: String,
    pub currency: String,
    /// `MM-DD`: the day the limit year opens.
    pub year_starts_on: String,
    pub note: Option<String>,
}

#[tauri::command]
pub fn goals_list(state: State<AppState>, date: String) -> UiResult<Vec<GoalRow>> {
    let as_of = parse_date(&date)?;
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let accounts = store.list_accounts()?;
    let analytics = PortfolioAnalytics::new(&store, &portfolio)?;

    store
        .list_goals(&portfolio.id)?
        .into_iter()
        .map(|goal| {
            let progress = analytics.goal_progress(&goal, as_of)?;
            Ok(GoalRow {
                account_names: goal
                    .accounts
                    .iter()
                    .filter_map(|id| accounts.iter().find(|a| &a.id == id))
                    .map(|a| a.name.clone())
                    .collect(),
                goal,
                progress,
            })
        })
        .collect()
}

#[tauri::command]
pub fn goal_save(app: AppHandle, state: State<AppState>, input: GoalInput) -> UiResult<Goal> {
    let goal = {
        let store = state.store()?;
        let portfolio = state.portfolio()?.clone();
        let mut goal = match &input.id {
            Some(id) => store.get_goal(id)?,
            None => Goal::new(&input.name, Decimal::ZERO, &input.currency, today()),
        };
        goal.name = require_text(&input.name, "name")?;
        goal.target_amount = amount(&input.target_amount, "target amount")?;
        goal.currency = sq_core::money::normalize_currency(&input.currency);
        goal.target_date = input.target_date.as_deref().map(parse_date).transpose()?;
        goal.monthly_amount = decimal(input.monthly_amount.as_deref(), "monthly amount")?;
        // The form asks for a percent; the calculation works in fractions, like every other rate.
        goal.expected_return = match decimal(input.expected_return.as_deref(), "expected return")? {
            Some(percent) => sq_core::calc::percent_to_rate(percent)
                .ok_or_else(|| UiError::invalid("the expected return is not a number"))?,
            None => Decimal::ZERO,
        };
        goal.note = input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        goal.accounts = input.accounts;
        store.save_goal(&portfolio.id, &goal)?;
        goal
    };

    emit_changed(&app, "goals")?;
    Ok(goal)
}

#[tauri::command]
pub fn goal_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_goal(&id)?;
    emit_changed(&app, "goals")
}

/// Every limit with what has been paid into its account over the limit year `date` falls in.
#[tauri::command]
pub fn limits_list(state: State<AppState>, date: String) -> UiResult<Vec<LimitUsage>> {
    let as_of = parse_date(&date)?;
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let analytics = PortfolioAnalytics::new(&store, &portfolio)?;

    store
        .list_limits()?
        .into_iter()
        .map(|limit| Ok(analytics.limit_usage(&limit, as_of)?))
        .collect()
}

#[tauri::command]
pub fn limit_save(app: AppHandle, state: State<AppState>, input: LimitInput) -> UiResult<ContributionLimit> {
    let limit = {
        let store = state.store()?;
        let mut limit = match &input.id {
            Some(id) => store
                .list_limits()?
                .into_iter()
                .find(|l| &l.id == id)
                .ok_or_else(|| UiError::invalid(format!("no limit {id}")))?,
            None => ContributionLimit::new(&input.account_id, &input.name, Decimal::ZERO, &input.currency),
        };
        limit.account_id = input.account_id;
        limit.name = require_text(&input.name, "name")?;
        limit.amount = amount(&input.amount, "amount")?;
        limit.currency = sq_core::money::normalize_currency(&input.currency);
        limit.year_starts_on = input.year_starts_on.trim().to_string();
        limit.note = input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        store.save_limit(&limit)?;
        limit
    };

    emit_changed(&app, "goals")?;
    Ok(limit)
}

#[tauri::command]
pub fn limit_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_limit(&id)?;
    emit_changed(&app, "goals")
}

/// A figure the form sent; an empty one is missing, not zero.
fn amount(value: &str, what: &str) -> UiResult<Decimal> {
    decimal(Some(value), what)?.ok_or_else(|| UiError::invalid(format!("{what} is required")))
}
