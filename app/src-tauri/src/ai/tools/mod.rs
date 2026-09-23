//! The catalogue: one tool = one read of the portfolio = one `calc` call. A tool body is the
//! body of the matching command with a different source of arguments — parse, take the scope,
//! call `calc`, shape the answer. No arithmetic happens here, the same rule `commands/` follows.
//!
//! Three rules hold for every entry:
//!
//! 1. **Readable identifiers, not UUIDs.** A row names a ticker, an account, a node — an id the
//!    model echoes back into a sentence must mean something to the person reading it.
//! 2. **Numbers as strings**, exactly as they cross IPC (`rust_decimal::serde::str`): the model
//!    is handed the same decimals the screen shows, not a lossy float.
//! 3. **The model never does date arithmetic.** A window is a `period` id resolved here through
//!    the same axis the UI uses (ADR-0018), never a pair of dates the model made up.
//!
//! Takes a `Store` and a `ScopeSelection`, never `AppState` — so the same bodies can back an MCP
//! server later without being rewritten.
//!
//! One file per namespace, each holding both the entries it declares and the bodies behind them:
//! a tool's schema is what the model is promised and its body is what delivers, and the two
//! drifting apart is the failure this layout exists to prevent. `CATALOGUE` below stays the one
//! index, in the order the model sees. `args`, `lookup` and `fmt` are the shared floor: reading
//! the model's arguments, resolving a name the user would recognise, and rounding an answer.

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
mod plans;
mod portfolio;
mod reports;
mod securities;
mod taxonomy;
mod transactions;
mod watchlist;

/// The dashboard's summary tile gathers these three itself: `ai_brief` sends the readings
/// fenced, with no tools at all, so the model chooses nothing (ADR-0039).
pub(super) use portfolio::{performance_over, portfolio_overview, positions_list};

/// What a tool body is given: the data, the lens over it, what "today" means, and how to say
/// that something was written.
///
/// `changed` is the host's own `data:changed` notification behind a closure, so a write tool
/// reaches the screens the same way a command does without this file naming the host. A write
/// that nobody is told about is a screen showing yesterday's portfolio until the user reloads —
/// and for a new instrument or a new trade the host also uses it to fetch the quotes nothing has
/// asked for yet (`.claude/rules/money-and-fx.md`).
pub struct ToolContext<'a> {
    pub store: &'a Store,
    pub scope: &'a ScopeSelection,
    pub today: NaiveDate,
    /// Called with one of the host's change scopes ("transactions", "securities", ...).
    pub changed: &'a dyn Fn(&'static str),
}

/// Whether the user is asked before this tool runs.
///
/// The three are not degrees of the same thing. `Free` reads no portfolio data at all. `Ask` is
/// a read: the user is asked the first time, and both a `session` grant and a chat's `AUTO` mode
/// stand in for that answer afterwards. `Write` changes the user's data, and **no grant of any
/// kind applies to it, now or ever** — every write call is confirmed on its own, and its card
/// states the change rather than the intention (ADR-0037).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Free,
    Ask,
    Write,
}

/// What the consent card is built from. Codes and values, never a sentence: the host does not
/// know the user's language and the model must not write the text on a button the user presses.
/// See `.claude/rules/ui-boundary.md` and ADR-0023.
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

/// The argument every tool the user is asked about carries: the model's own one-line answer to
/// "why do you want this". It is *not* a tool argument — no body reads it — but it travels as one
/// because that is the only channel a model has beside its answer text, and because a reason
/// written in the same call cannot drift from the call it explains.
pub const REASON: &str = "reason";

/// The catalogue in the shape a provider adapter turns into its own `tools` field.
///
/// `reason` is added here rather than in 62 schemas: one rule, one place, and a tool written
/// tomorrow gets it without its author remembering to. `Access::Free` is left alone — nobody is
/// asked about a tool that reads no portfolio data, so a sentence explaining it is paid for and
/// never read.
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
fn with_reason(mut schema: Value) -> Value {
    let described = json!({
        "type": "string",
        "description": "One short sentence, in the user's language, saying why you need this \
                        for what they asked. It is shown to them before they allow the call."
    });
    if let Some(properties) = schema["properties"].as_object_mut() {
        properties.insert(REASON.to_string(), described);
    }
    if let Some(required) = schema["required"].as_array_mut() {
        required.push(Value::String(REASON.to_string()));
    }
    schema
}

/// Anything a tool body fails at is reported back to the model as a tool error, never as a
/// failed turn: a miss it can correct is an answer, not a crash.
pub(super) fn tool(e: impl std::fmt::Display) -> AiError {
    AiError::Tool(e.to_string())
}

#[cfg(test)]
mod tests;
