use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};
use sq_core::calc::Period;
use sq_core::market::DateRange;

pub(super) const PORTFOLIO_OVERVIEW: Tool = Tool {
    name: "portfolio_overview",
    description: "Total value, cash and the day's change for the portfolio the user is \
                  currently looking at. Start here when the question is about the portfolio \
                  as a whole.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: portfolio_overview,
};

pub(super) const PORTFOLIO_PERFORMANCE: Tool = Tool {
    name: "portfolio_performance",
    description: "Time-weighted return, money-weighted return (XIRR), best and worst period, \
                  and the costs paid, over one period. Use this for \"how did I do\".",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: portfolio_performance,
};

pub(super) const PORTFOLIO_RISK: Tool = Tool {
    name: "portfolio_risk",
    description: "Volatility, Sharpe ratio and maximum drawdown over one period.",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: portfolio_risk,
};

pub(super) const POSITIONS_LIST: Tool = Tool {
    name: "positions_list",
    description: "The instruments held today: quantity, price, value, weight, unrealised \
                  result and the day's change. Ordered by value, largest first.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "limit": { "type": ["integer", "null"], "description": "How many rows, largest first. Null means all." }
            },
            "required": ["limit"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("limit", limit_of(args).map_or("all".into(), |n| n.to_string())),
    run: positions_list,
};

pub(super) const PORTFOLIO_BENCHMARK: Tool = Tool {
    name: "portfolio_benchmark",
    description: "The portfolio's return over a period beside one instrument's, and the \
                  difference between them. The instrument stands in for the market — an index \
                  fund the user holds or merely tracks.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument to compare against, by ticker." },
                "period": period_property()
            },
            "required": ["symbol", "period"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert("instrument".into(), text(args, "symbol"));
        params
    },
    run: portfolio_benchmark,
};

pub(super) const POSITIONS_RETURNS: Tool = Tool {
    name: "positions_returns",
    description: "Return per instrument over a period: time-weighted return, money-weighted \
                  return, result in base currency and what each contributed to the \
                  portfolio's own return. positions_list answers \"what do I hold\"; this one \
                  answers \"what did each of them earn\".",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "period": period_property(),
                "limit": { "type": ["integer", "null"], "description": "How many rows, largest contribution first. Null means all." }
            },
            "required": ["period", "limit"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert(
            "limit".into(),
            limit_of(args).map_or("all".into(), |n| n.to_string()),
        );
        params
    },
    run: positions_returns,
};

pub(super) const ACCOUNTS_LIST: Tool = Tool {
    name: "accounts_list",
    description: "The accounts in the current scope with their kind and cash balances.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: accounts_list,
};

pub(in crate::ai) fn portfolio_overview(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let valuation = analytics.valuation_at(context.today).map_err(tool)?;

    Ok(json!({
        "date": context.today.to_string(),
        "base_currency": analytics.base_currency(),
        "total_value": money(valuation.total_value_base),
        "securities_value": money(valuation.securities_value_base),
        "cash": money(valuation.cash_base),
        "unrealized_result": money(valuation.unrealized_pnl_base),
        "positions": valuation.positions.len(),
        "accounts": analytics.scope_accounts().len(),
    }))
}

pub(super) fn portfolio_performance(context: &ToolContext, args: &Value) -> AiResult<Value> {
    performance_over(context, resolve_period(context, args)?)
}

/// The window arrives resolved here: the model names a period, but the dashboard brief already
/// holds the dates the tile was drawn for, and resolving them twice could disagree.
pub(in crate::ai) fn performance_over(context: &ToolContext, range: DateRange) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let series = analytics.series(range).map_err(tool)?;
    let summary = sq_core::calc::period_summary(&series);
    let costs = analytics.costs(range.from, range.to).map_err(tool)?;
    let twr = analytics.twr(range.from, range.to).map_err(tool)?;

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "twr_percent": percent(twr),
        // A window shorter than a day has no annual rate to state; absent, not zero.
        "twr_annualized_percent": match sq_core::calc::annualize(twr, range.from, range.to) {
            Some(rate) => percent(rate),
            None => Value::Null,
        },
        // A portfolio whose cash flows do not bracket a sign has no internal rate of return;
        // that is an answer, not a failure, so it is reported as absent rather than erroring.
        "xirr_percent": match analytics.xirr(range.to) {
            Ok(v) => percent(v),
            Err(_) => Value::Null,
        },
        // Absent when the portfolio names no price-index region: a setting, not a gap. The
        // window can end earlier than the period, so it is carried rather than assumed.
        "real": match analytics.real_performance(range.from, range.to) {
            Ok(Some(real)) => json!({
                "region": real.twr.region,
                "deflated_through": real.twr.to.to_string(),
                "inflation_percent": percent(real.twr.inflation),
                "twr_percent": percent(real.twr.real),
                "twr_annualized_percent": match real.twr.real_annualized {
                    Some(rate) => percent(rate),
                    None => Value::Null,
                },
                "xirr_percent": match real.xirr {
                    Some(v) => percent(v),
                    None => Value::Null,
                },
            }),
            // A named region whose index has not arrived yet leaves the rest of the answer standing.
            Ok(None) | Err(_) => Value::Null,
        },
        "start_value": money(summary.start_value_base),
        "end_value": money(summary.end_value_base),
        "net_contributions": money(summary.net_flow_base),
        "earned": money(summary.delta_base),
        "fees": money(costs.fees_base),
        "taxes": money(costs.taxes_base),
        "annual_returns": analytics
            .returns_by_period(range, Period::Year)
            .map_err(tool)?
            .iter()
            .map(|r| json!({
                "from": r.from.to_string(),
                "to": r.to.to_string(),
                "return_percent": percent(r.twr),
            }))
            .collect::<Vec<_>>(),
    }))
}

pub(super) fn portfolio_risk(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    // The same defaults the Risk screen opens with: a zero risk-free rate states no assumption,
    // and 30 days is the shipped rolling window.
    let report = analytics.risk_report(range, 0.0, 30).map_err(tool)?;

    // Fewer than two daily returns is not a drawdown of zero — there is simply no episode.
    let deepest = report.metrics.max_drawdown.as_ref();
    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "days": report.metrics.days,
        "volatility_percent": ratio(report.metrics.volatility * 100.0, 2),
        "sharpe": report.metrics.sharpe.map(|v| ratio(v, 2)),
        "max_drawdown_percent": deepest.map(|d| ratio(d.depth * 100.0, 2)),
        "max_drawdown_trough": deepest.map(|d| d.trough.to_string()),
        "max_drawdown_recovered": deepest.and_then(|d| d.recovered.map(|r| r.to_string())),
        "current_drawdown_percent": ratio(report.metrics.current_drawdown * 100.0, 2),
        "risk_free_rate_percent": 0.0,
    }))
}

pub(in crate::ai) fn positions_list(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let holdings = analytics.holdings_at(context.today).map_err(tool)?;
    let valuation = sq_core::calc::value_holdings(
        &holdings,
        analytics.base_currency(),
        context.today,
        context.store,
        context.store,
    )
    .map_err(tool)?;

    let securities = context.store.list_securities().map_err(tool)?;
    let total = valuation.total_value_base;
    let mut rows: Vec<_> = valuation.positions.iter().collect();
    rows.sort_by_key(|p| std::cmp::Reverse(p.market_value_base));
    if let Some(limit) = limit_of(args) {
        rows.truncate(limit);
    }

    let rows: Vec<Value> = rows
        .iter()
        .map(|p| {
            let security = securities.iter().find(|s| s.id == p.security_id);
            json!({
                "symbol": security.map(|s| s.symbol.as_str()).unwrap_or("?"),
                "name": security.map(|s| s.name.as_str()).unwrap_or("?"),
                "currency": p.currency,
                "quantity": quantity(p.quantity),
                "price": price(p.price),
                "value": money(p.market_value_base),
                "cost": money(p.cost_basis_base),
                "unrealized_result": money(p.unrealized_pnl_base),
                "weight_percent": percent(if total.is_zero() {
                    rust_decimal::Decimal::ZERO
                } else {
                    p.market_value_base / total
                }),
            })
        })
        .collect();

    Ok(json!({
        "date": context.today.to_string(),
        "base_currency": analytics.base_currency(),
        "total_value": money(total),
        "shown": rows.len(),
        "held": valuation.positions.len(),
        "rows": rows,
    }))
}

pub(super) fn accounts_list(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let balances = analytics.cash_balances(context.today).map_err(tool)?;

    let rows: Vec<Value> = analytics
        .scope_accounts()
        .iter()
        .map(|account| {
            json!({
                "name": account.name,
                "kind": format!("{:?}", account.kind),
                "currency": account.currency,
                "cash": balances
                    .get(&account.id)
                    .map(|per_currency| per_currency
                        .iter()
                        .map(|(currency, amount)| json!({ "currency": currency, "amount": money(*amount) }))
                        .collect::<Vec<_>>())
                    .unwrap_or_default(),
            })
        })
        .collect();

    Ok(json!({ "base_currency": analytics.base_currency(), "accounts": rows }))
}

/// The comparison reports the window it actually covered: an index younger than the period is
/// compared over the overlap, and both legs are narrowed together (`.claude/rules/money-and-fx.md`).
pub(super) fn portfolio_benchmark(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let compared = analytics
        .benchmark(&security.id, range.from, range.to)
        .map_err(tool)?;

    Ok(json!({
        "instrument": security.symbol,
        "name": security.name,
        "asked_from": range.from.to_string(),
        "from": compared.from.to_string(),
        "to": compared.to.to_string(),
        "base_currency": analytics.base_currency(),
        "portfolio_twr_percent": percent(compared.portfolio_twr),
        "benchmark_twr_percent": percent(compared.benchmark_twr),
        "excess_percent": percent(compared.excess),
        // A benchmark with no price at the start of the period shortens the comparison rather
        // than forging it, so say when the window moved.
        "window_shortened": compared.from > range.from,
    }))
}

pub(super) fn positions_returns(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let mut rows = analytics.position_returns(range.from, range.to).map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    rows.sort_by_key(|r| std::cmp::Reverse(r.contribution));
    let held = rows.len();
    if let Some(limit) = limit_of(args) {
        rows.truncate(limit);
    }

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "shown": rows.len(),
        "instruments": held,
        "rows": rows
            .iter()
            .map(|row| {
                let security = securities.iter().find(|s| s.id == row.security_id);
                json!({
                    "symbol": security.map(|s| s.symbol.as_str()).unwrap_or("?"),
                    "name": security.map(|s| s.name.as_str()).unwrap_or("?"),
                    // Absent where the position had no capital at work to divide by — not zero.
                    "twr_percent": row.twr.map(percent),
                    "twr_annualized_percent": row.twr_annualized.map(percent),
                    "xirr_percent": row.xirr.map(percent),
                    "result": money(row.pnl_base),
                    "return_on_money_percent": row.absolute_performance.map(percent),
                    "fees": money(row.fees_base),
                    "taxes": money(row.taxes_base),
                    "contribution_percent": percent(row.contribution),
                })
            })
            .collect::<Vec<_>>(),
    }))
}
