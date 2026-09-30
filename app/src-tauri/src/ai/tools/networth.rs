//! Net worth: the portfolio plus what else is owned, minus what is owed. A second total, so the
//! answer names it apart from the portfolio's own value and carries no return (ADR-0092).

use super::args::*;
use super::fmt::*;
use super::{Access, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const NET_WORTH: Tool = Tool {
    name: "net_worth",
    description: "Everything the user owns and owes, not only what is invested: property, \
                  vehicles, valuables, cash held elsewhere, and debts such as a mortgage or a \
                  loan. Each figure is the user's own dated valuation, not a market price, so \
                  none of it has a return, a volatility or a place in the allocation — use \
                  portfolio_overview for the invested part. Ignores the account lens.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: net_worth,
};

fn net_worth(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let portfolio = &context.scope.portfolio;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let reading = analytics.net_worth(context.today).map_err(tool)?;
    let assets = context.store.list_assets(&portfolio.id).map_err(tool)?;

    let holdings: Vec<Value> = reading
        .holdings
        .iter()
        .map(|holding| {
            json!({
                "name": holding.name,
                "kind": holding.kind.as_str(),
                "side": match holding.side {
                    sq_core::model::AssetSide::Owned => "owned",
                    sq_core::model::AssetSide::Owed => "owed",
                },
                "value": money(holding.amount),
                "currency": holding.currency,
                "value_base": money(holding.amount_base),
                // The day the user last said so. An old day means an old opinion, and the model
                // should say so rather than treat the figure as today's.
                "valued_on": holding.valued_on.to_string(),
                "days_old": holding.days_old,
                "figure_is_old": holding.stale,
                "change_since_previous": holding.change_base.map(money),
                "previous_figure_from": holding.changed_since.map(|d| d.to_string()),
                "secured_by": holding
                    .secured_by
                    .as_deref()
                    .and_then(|id| assets.iter().find(|a| a.id == id))
                    .map(|a| a.name.as_str()),
                "debt_against_it": holding.secured_debt_base.map(money),
                "equity": holding.equity_base.map(money),
                "payoff": holding.payoff.as_ref().map(|payoff| json!({
                    // Absent means the payment does not cover the interest, so there is no end —
                    // which is not the same as ending today.
                    "months_left": payoff.months_left,
                    "payoff_on": payoff.payoff_on.map(|d| d.to_string()),
                    "interest_still_to_pay": payoff.interest_ahead.map(money),
                    "contract_ends_on": payoff.ends_on.map(|d| d.to_string()),
                    "paid_off_percent": payoff.paid_share.map(percent),
                })),
            })
        })
        .collect();

    // Named, not silently dropped: a house with no valuation yet is missing from the sum, and
    // the difference between "worth nothing" and "never valued" matters to the answer.
    let waiting: Vec<&str> = reading
        .not_valued_yet
        .iter()
        .filter_map(|id| assets.iter().find(|a| &a.id == id))
        .map(|a| a.name.as_str())
        .collect();

    Ok(json!({
        "date": reading.date.to_string(),
        "base_currency": reading.base_currency,
        "investments": money(reading.investments_base),
        "other_assets": money(reading.owned_base),
        "liabilities": money(reading.owed_base),
        "net_worth": money(reading.net_base),
        "invested_percent": reading.invested_share.map(percent),
        "debt_to_assets_percent": reading.debt_to_assets.map(percent),
        "figures_older_than_half_a_year": reading.stale_count,
        "items": holdings,
        "not_valued_yet": waiting,
    }))
}
