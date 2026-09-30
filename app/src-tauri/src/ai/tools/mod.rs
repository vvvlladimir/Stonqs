//! The catalogue: one tool = one `calc` call, no arithmetic here. Readable identifiers, numbers as
//! strings, and periods resolved by id (ADR-0018). A schema sits beside its body; `CATALOGUE` is the index.

use super::{AiError, AiResult};
use crate::state::ScopeSelection;
use chrono::NaiveDate;
use serde_json::{Value, json};
use sq_core::storage::Store;
use std::collections::BTreeMap;

mod accounts;
mod alerts;
mod allocation;
mod app;
mod args;
mod fmt;
mod lookup;
mod market;
mod networth;
mod plans;
pub mod plugin;
mod portfolio;
mod reports;
mod securities;
mod taxonomy;
mod transactions;
mod watchlist;

/// The dashboard's summary tile gathers these three itself: `ai_brief` sends the readings
/// fenced, with no tools at all, so the model chooses nothing (ADR-0039).
pub(super) use portfolio::{performance_over, portfolio_overview, positions_list};

/// `changed` is the host's `data:changed` behind a closure, which also triggers the missing-quote
/// fetch for a new instrument or trade.
pub struct ToolContext<'a> {
    pub store: &'a Store,
    pub scope: &'a ScopeSelection,
    pub today: NaiveDate,
    /// Which source prices an instrument the model creates or re-points; `None` when the owner
    /// has switched none on, and then the instrument is priced by hand (ADR-0076).
    pub quotes_source: Option<&'a str>,
    /// Called with one of the host's change scopes ("transactions", "securities", ...).
    pub changed: &'a dyn Fn(&'static str),
}

/// `Free` reads no portfolio data; `Ask` is a read a grant or `AUTO` may cover; `Write` is
/// confirmed every time, with no grant ever applying (ADR-0037).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Free,
    Ask,
    Write,
}

/// Codes and values for the consent card, never a sentence (ADR-0023).
pub type Params = BTreeMap<String, String>;

pub struct Tool {
    pub name: &'static str,
    /// Written for the model, not the user: what question this answers, and when to prefer it.
    pub description: &'static str,
    pub access: Access,
    /// JSON Schema of the arguments, `strict`-compatible.
    pub schema: fn() -> Value,
    /// The values behind the consent card, already resolved (dates, not period codes).
    pub summary: fn(&ToolContext, &Value) -> Params,
    pub run: fn(&ToolContext, &Value) -> AiResult<Value>,
}

/// The whole catalogue, in the order the model sees it.
pub const CATALOGUE: &[Tool] = &[
    // Reads, in the order a question usually travels: the portfolio, then what is in it, then
    // one instrument, then the plans and lists around it.
    portfolio::PORTFOLIO_OVERVIEW,
    portfolio::PORTFOLIO_PERFORMANCE,
    portfolio::PORTFOLIO_BREAKDOWN,
    portfolio::PORTFOLIO_RISK,
    portfolio::PORTFOLIO_BENCHMARK,
    portfolio::POSITIONS_LIST,
    portfolio::POSITIONS_RETURNS,
    portfolio::ACCOUNTS_LIST,
    networth::NET_WORTH,
    accounts::ACCOUNT_GROUPS_LIST,
    allocation::ALLOCATION_TREES,
    allocation::ALLOCATION_BREAKDOWN,
    taxonomy::TAXONOMY_TREE,
    transactions::INCOME_SUMMARY,
    transactions::INCOME_CALENDAR,
    transactions::INCOME_BREAKDOWN,
    transactions::TRANSACTIONS_LIST,
    securities::SECURITIES_FIND,
    plans::PLANS_LIST,
    plans::PLANS_PROJECTION,
    plans::PLANS_DUE,
    plans::PLANS_GOALS,
    plans::ACCOUNTS_LIMITS,
    alerts::ALERTS_LIST,
    alerts::ALERTS_CROSSINGS,
    securities::SECURITIES_EVENTS,
    market::MARKET_QUOTES,
    watchlist::WATCHLISTS_LIST,
    watchlist::WATCHLIST_ROWS,
    allocation::ALLOCATION_MEMBERS,
    allocation::REBALANCE_TARGETS,
    allocation::REBALANCE_PLAN,
    reports::REPORT_CAPITAL_GAINS,
    reports::REPORT_CHARGES,
    reports::REPORT_TRADES,
    reports::REPORT_SUMMARY,
    // Writes. Every one of them asks the user each time it is called, whatever the chat's
    // standing permission says, so their order here is only the order the model reads them in.
    securities::SECURITY_CREATE,
    securities::SECURITY_DELETE,
    securities::SECURITY_SET_DATA_SOURCE,
    securities::SECURITY_SET_LISTING,
    securities::SECURITY_SET_NOTE,
    securities::SECURITY_NOTE_ADD,
    securities::SECURITY_NOTE_DELETE,
    alerts::ALERT_CREATE,
    alerts::ALERT_DELETE,
    transactions::TRANSACTION_CREATE,
    transactions::TRANSACTION_UPDATE,
    transactions::TRANSACTION_DELETE,
    accounts::ACCOUNT_CREATE,
    accounts::ACCOUNT_UPDATE,
    accounts::ACCOUNT_DELETE,
    accounts::ACCOUNT_GROUP_SET,
    accounts::ACCOUNT_GROUP_DELETE,
    plans::PLAN_SAVE,
    plans::PLAN_DELETE,
    plans::PLAN_COMMIT,
    watchlist::WATCHLIST_SET,
    watchlist::WATCHLIST_DELETE,
    taxonomy::TAXONOMY_CREATE,
    taxonomy::TAXONOMY_DELETE,
    taxonomy::TAXONOMY_NODE_ADD,
    taxonomy::TAXONOMY_NODE_DELETE,
    taxonomy::TAXONOMY_ASSIGN,
    taxonomy::TAXONOMY_UNASSIGN,
    taxonomy::TAXONOMY_EXCLUDE,
    allocation::REBALANCE_TARGET_SAVE,
    allocation::REBALANCE_TARGET_DELETE,
    // The documentation. Free: it reads no portfolio data, so there is nothing to consent to.
    app::APP_PERIODS,
    app::APP_REFERENCE,
    app::APP_USER_GUIDE,
];

pub fn find(name: &str) -> Option<&'static Tool> {
    CATALOGUE.iter().find(|t| t.name == name)
}

/// The model's one-line reason, injected into every asking tool's schema; no body reads it.
pub const REASON: &str = "reason";

/// Adds `reason` to every asking tool in one place; `Free` tools are left alone.
pub fn definitions() -> Vec<(&'static str, &'static str, Value)> {
    CATALOGUE
        .iter()
        .map(|t| {
            let schema = match t.access {
                Access::Free => (t.schema)(),
                _ => with_reason((t.schema)()),
            };
            (t.name, t.description, schema)
        })
        .collect()
}

/// `strict` requires every property to be listed as required, so the field is added to both.
pub(super) fn with_reason(mut schema: Value) -> Value {
    let described = json!({
        "type": "string",
        "description": "One short sentence, in the user's language, saying why you need this \
                        for what they asked. It is shown to them before they allow the call."
    });
    add_required(&mut schema, REASON, described);
    schema
}

/// Adds a property and lists it as required, creating the array when the schema has none: a
/// property left out of `required` makes a strict provider refuse the whole request.
pub(super) fn add_required(schema: &mut Value, name: &str, property: Value) {
    if let Some(properties) = schema["properties"].as_object_mut() {
        properties.insert(name.to_string(), property);
    }
    if !schema["required"].is_array() {
        schema["required"] = json!([]);
    }
    if let Some(required) = schema["required"].as_array_mut() {
        required.push(Value::String(name.to_string()));
    }
}

/// Anything a tool body fails at is reported back to the model as a tool error, never as a
/// failed turn: a miss it can correct is an answer, not a crash.
pub(super) fn tool(e: impl std::fmt::Display) -> AiError {
    AiError::Tool(e.to_string())
}

#[cfg(test)]
mod tests;
