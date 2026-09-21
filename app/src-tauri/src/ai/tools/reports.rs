use super::args::*;
use super::fmt::*;
use super::{Access, AiResult, Tool, ToolContext, tool};
use chrono::{Duration, NaiveDate};
use serde_json::{Value, json};

pub(super) const REPORT_CAPITAL_GAINS: Tool = Tool {
    name: "report_capital_gains",
    description: "What selling actually realised over a period: proceeds, cost, fees, taxes \
                  and the result, in total and per instrument. Unrealised results are in \
                  positions_list instead.",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: report_capital_gains,
};

pub(super) const REPORT_CHARGES: Tool = Tool {
    name: "report_charges",
    description: "The fees and taxes billed over a period, split by what they were for and \
                  which account paid them. Commission inside a purchase is part of its cost \
                  and is not counted here.",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: report_charges,
};

pub(super) const REPORT_TRADES: Tool = Tool {
    name: "report_trades",
    description: "Trading over a period: what was bought and sold, how the trades closed in \
                  the window turned out, how long they were held and what is still open. A \
                  trade is the whole round trip, not one row of the ledger.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "period": period_property(),
                "limit": { "type": ["integer", "null"], "description": "How many closed trades to list, most recent first. Null means 20." }
            },
            "required": ["period", "limit"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert("limit".into(), limit_of(args).unwrap_or(TRADES_CAP).to_string());
        params
    },
    run: report_trades,
};

pub(super) const REPORT_SUMMARY: Tool = Tool {
    name: "report_summary",
    description: "Realised result, income and charges over a period, each beside the same \
                  figure over the window immediately before it. Use it for \"is this better \
                  than last year\" — the comparison is the point.",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: report_summary,
};

pub(super) fn report_capital_gains(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let holdings = analytics.holdings_at(range.to).map_err(tool)?;
    let realized = sq_core::calc::realized_between(&holdings, range.from, range.to);
    let total = sq_core::calc::capital_gains_total(&realized);
    let securities = context.store.list_securities().map_err(tool)?;

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "disposals": total.disposals,
        "proceeds": money(total.proceeds_base),
        "cost": money(total.cost_base),
        "fees": money(total.fees_base),
        "taxes": money(total.taxes_base),
        "result": money(total.gain_base),
        // The exchange rate's share of the result, each disposal at its own rate (ADR-0028).
        "currency_share_of_result": money(total.currency_gain_base),
        "by_instrument": sq_core::calc::capital_gains_by_security(&realized)
            .iter()
            .map(|(id, summary)| json!({
                "instrument": securities
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.symbol.as_str())
                    .unwrap_or("?"),
                "disposals": summary.disposals,
                "result": money(summary.gain_base),
                "return_on_cost_percent": summary.return_on_cost().map(percent),
            }))
            .collect::<Vec<_>>(),
        "by_year": sq_core::calc::capital_gains_by_year(&realized)
            .iter()
            .map(|(year, summary)| json!({
                "year": year,
                "disposals": summary.disposals,
                "result": money(summary.gain_base),
            }))
            .collect::<Vec<_>>(),
    }))
}

pub(super) fn report_charges(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let holdings = analytics.holdings_at(range.to).map_err(tool)?;
    // Standalone fees and taxes only: commission paid inside a purchase is part of its cost
    // basis and counting it here would count it twice (ADR-0024).
    let charges = sq_core::calc::charges_between(&holdings, range.from, range.to);
    let total = sq_core::calc::charges_total(&charges);
    let accounts = context.store.list_accounts().map_err(tool)?;

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "charges": total.count,
        "fees": money(total.fees_base),
        "taxes": money(total.taxes_base),
        "by_kind": sq_core::calc::charges_by_kind(&charges)
            .iter()
            .map(|(kind, summary)| json!({
                "kind": format!("{kind:?}"),
                "count": summary.count,
                "fees": money(summary.fees_base),
                "taxes": money(summary.taxes_base),
            }))
            .collect::<Vec<_>>(),
        "by_account": sq_core::calc::charges_by_account(&charges)
            .iter()
            .map(|(id, summary)| json!({
                "account": accounts
                    .iter()
                    .find(|a| &a.id == id)
                    .map(|a| a.name.as_str())
                    .unwrap_or("?"),
                "fees": money(summary.fees_base),
                "taxes": money(summary.taxes_base),
            }))
            .collect::<Vec<_>>(),
    }))
}

/// How many closed trades one answer lists. The rest are in the totals above them.
pub(super) const TRADES_CAP: usize = 20;

pub(super) fn report_trades(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let book = analytics.trades(range.to).map_err(tool)?;
    let volume = analytics.trading_volume(range.from, range.to).map_err(tool)?;
    let summary = analytics.period_summary(range).map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    // A trade belongs to the window its disposal fell in; the open ones are as of `to`.
    let mut closed: Vec<sq_core::calc::Trade> = book
        .closed
        .into_iter()
        .filter(|t| {
            t.closed_at
                .is_some_and(|date| date >= range.from && date <= range.to)
        })
        .collect();
    let closed_stats = sq_core::calc::trade_stats(&closed);
    let open_stats = sq_core::calc::trade_stats(&book.open);
    closed.sort_by_key(|t| std::cmp::Reverse(t.closed_at));
    closed.truncate(limit_of(args).unwrap_or(TRADES_CAP).min(TRADES_CAP));

    let name_of = |id: &str| {
        securities
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.symbol.clone())
            .unwrap_or_else(|| "?".to_string())
    };

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "bought": money(volume.bought_base),
        "sold": money(volume.sold_base),
        // Volume against the capital at work: what the portfolio's own trading amounts to.
        "turnover_percent": summary.rate_of(volume.volume_base).map(percent),
        "closed": trade_stats_json(&closed_stats),
        "open": trade_stats_json(&open_stats),
        "closed_trades": closed
            .iter()
            .map(|trade| json!({
                "instrument": name_of(&trade.security_id),
                "opened": trade.opened_at.to_string(),
                "closed": trade.closed_at.map(|d| d.to_string()),
                "quantity": quantity(trade.quantity),
                "result": money(trade.pnl_base),
                "return_percent": trade.return_pct.map(percent),
                "annualized_percent": trade.irr.map(percent),
                "held_days": trade.holding_days,
            }))
            .collect::<Vec<_>>(),
    }))
}

fn trade_stats_json(stats: &sq_core::calc::TradeStats) -> Value {
    json!({
        "trades": stats.trades,
        "winners": stats.winners,
        "losers": stats.losers,
        "result": money(stats.pnl_base),
        "entry_value": money(stats.entry_value_base),
        "exit_value": money(stats.exit_value_base),
        "average_held_days": stats.average_holding_days,
        // Absent when there is no trade to divide by, which is not a win rate of zero.
        "win_rate_percent": stats.win_rate.map(percent),
    })
}

/// The window immediately before `[from, to]`, of the same length — what "versus the previous
/// window" means on the reports screen, and the same arithmetic `reports_summary` does.
fn previous_window(from: NaiveDate, to: NaiveDate) -> (NaiveDate, NaiveDate) {
    let span = (to - from) + Duration::days(1);
    let previous_to = from - Duration::days(1);
    (previous_to - span + Duration::days(1), previous_to)
}

pub(super) fn report_summary(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    // One build of the holdings covers both windows: the earlier one ends before `from`.
    let holdings = analytics.holdings_at(range.to).map_err(tool)?;
    let (was_from, was_to) = previous_window(range.from, range.to);

    let gains =
        sq_core::calc::capital_gains_total(&sq_core::calc::realized_between(&holdings, range.from, range.to));
    let gains_was =
        sq_core::calc::capital_gains_total(&sq_core::calc::realized_between(&holdings, was_from, was_to));
    let income =
        sq_core::calc::dividends_total(&sq_core::calc::income_between(&holdings, range.from, range.to));
    let income_was =
        sq_core::calc::dividends_total(&sq_core::calc::income_between(&holdings, was_from, was_to));
    let charges =
        sq_core::calc::charges_total(&sq_core::calc::charges_between(&holdings, range.from, range.to));
    let charges_was =
        sq_core::calc::charges_total(&sq_core::calc::charges_between(&holdings, was_from, was_to));

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "previous_from": was_from.to_string(),
        "previous_to": was_to.to_string(),
        "base_currency": analytics.base_currency(),
        "realized_result": money(gains.gain_base),
        "realized_result_before": money(gains_was.gain_base),
        "disposals": gains.disposals,
        "income": money(income.net_base),
        "income_before": money(income_was.net_base),
        "payments": income.payments,
        "charges": money(charges.fees_base + charges.taxes_base),
        "charges_before": money(charges_was.fees_base + charges_was.taxes_base),
        "fees": money(charges.fees_base),
        "taxes": money(charges.taxes_base),
    }))
}
