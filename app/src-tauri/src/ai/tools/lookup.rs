//! Turning a name the user would recognise into the entity behind it. Every miss is a tool
//! error naming what was not found, so the model corrects itself instead of the turn failing.

use super::{AiError, AiResult, ToolContext, tool};

/// Write tools name an instrument the way the user does, by ticker — the model never sees an id.
pub(super) fn security_by_symbol(context: &ToolContext, symbol: &str) -> AiResult<sq_core::model::Security> {
    context
        .store
        .list_securities()
        .map_err(tool)?
        .into_iter()
        .find(|s| s.symbol.eq_ignore_ascii_case(symbol))
        .ok_or_else(|| AiError::Tool(format!("no instrument called {symbol} in this portfolio")))
}

/// An account by the name the user gave it — never an id, which the model never sees.
pub(super) fn account_by_name(context: &ToolContext, name: &str) -> AiResult<sq_core::model::Account> {
    context
        .store
        .list_accounts()
        .map_err(tool)?
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| AiError::Tool(format!("no account called {name} in this portfolio")))
}

/// A tree is named by the user, so it is matched the way they would write it.
pub(super) fn taxonomy_by_name(context: &ToolContext, wanted: &str) -> AiResult<sq_core::model::Taxonomy> {
    context
        .store
        .list_taxonomies()
        .map_err(tool)?
        .into_iter()
        .find(|t| t.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no classification tree named {wanted}")))
}

/// A branch of one tree. Names are the user's, so the match is theirs too; a miss names the
/// tree as well, because "Europe" exists in several of them and only one is being asked about.
pub(super) fn node_by_name(
    context: &ToolContext,
    taxonomy: &sq_core::model::Taxonomy,
    wanted: &str,
) -> AiResult<sq_core::model::TaxonomyNode> {
    context
        .store
        .taxonomy_nodes(&taxonomy.id)
        .map_err(tool)?
        .into_iter()
        .find(|n| n.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no branch named {wanted} in {}", taxonomy.name)))
}

/// A target allocation of this portfolio, by the name the user gave it.
pub(super) fn target_by_name(
    context: &ToolContext,
    wanted: &str,
) -> AiResult<sq_core::model::AllocationTarget> {
    context
        .store
        .targets_for_portfolio(&context.scope.portfolio.id)
        .map_err(tool)?
        .into_iter()
        .find(|t| t.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no target allocation named {wanted}")))
}

/// A plan of this portfolio. Plans are not scoped, so this reads the portfolio, never the lens.
pub(super) fn plan_by_name(context: &ToolContext, wanted: &str) -> AiResult<sq_core::model::InvestmentPlan> {
    context
        .store
        .list_plans(&context.scope.portfolio.id)
        .map_err(tool)?
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no plan named {wanted}")))
}

pub(super) fn watchlist_by_name(context: &ToolContext, wanted: &str) -> AiResult<sq_core::model::Watchlist> {
    context
        .store
        .list_watchlists()
        .map_err(tool)?
        .into_iter()
        .find(|l| l.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no watchlist named {wanted}")))
}

pub(super) fn group_by_name(context: &ToolContext, wanted: &str) -> AiResult<sq_core::model::AccountGroup> {
    context
        .store
        .list_account_groups()
        .map_err(tool)?
        .into_iter()
        .find(|g| g.name.eq_ignore_ascii_case(wanted))
        .ok_or_else(|| AiError::Tool(format!("no account group named {wanted}")))
}
