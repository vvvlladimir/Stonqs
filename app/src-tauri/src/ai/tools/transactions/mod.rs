use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiResult, Tool, ToolContext, tool};
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod write;
pub(super) use write::{TRANSACTION_CREATE, TRANSACTION_DELETE, TRANSACTION_UPDATE};

pub(super) const INCOME_SUMMARY: Tool = Tool {
    name: "income_summary",
    description: "Dividends, interest and other income received over one period, per \
                  instrument and in total.",
    access: Access::Ask,
    schema: period_argument,
    summary: period_summary,
    run: income_summary,
};

pub(super) const INCOME_BREAKDOWN: Tool = Tool {
    name: "income_breakdown",
    description: "The same income split through one classification tree, by name as listed \
                  by allocation_trees: what each category paid over the period. A payment \
                  follows the classification of whoever paid it, so this answers \"where does \
                  my income come from\", not what each category is worth.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "period": period_property(),
                "tree": { "type": "string", "description": "The tree's name, exactly as allocation_trees returned it." }
            },
            "required": ["period", "tree"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert("tree".into(), text(args, "tree"));
        params
    },
    run: income_breakdown,
};

pub(super) const TRANSACTIONS_LIST: Tool = Tool {
    name: "transactions_list",
    description: "The ledger over one period: date, kind, instrument, quantity, amount. Use \
                  it for \"what did I buy\", never to compute a total the other tools give.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "period": period_property(),
                "limit": { "type": ["integer", "null"], "description": "How many rows, most recent first. Null means up to 200." }
            },
            "required": ["period", "limit"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert(
            "limit".into(),
            limit_of(args).map_or("200".into(), |n| n.to_string()),
        );
        params
    },
    run: transactions_list,
};

pub(super) const INCOME_CALENDAR: Tool = Tool {
    name: "income_calendar",
    description: "Income, costs and savings laid out month by month, quarter by quarter or \
                  year by year over a period, with what each payer contributed. Use it for \
                  \"when does the money come in\"; income_summary answers the total alone.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "period": period_property(),
                "interval": {
                    "type": ["string", "null"],
                    "enum": ["MONTH", "QUARTER", "YEAR", null],
                    "description": "How wide one column is. Null means months."
                }
            },
            "required": ["period", "interval"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert(
            "interval".into(),
            optional(args, "interval").unwrap_or_else(|| "MONTH".into()),
        );
        params
    },
    run: income_calendar,
};

pub(super) fn income_summary(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let records = analytics.income(range.from, range.to).map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let mut total = rust_decimal::Decimal::ZERO;
    let mut by_instrument: BTreeMap<String, rust_decimal::Decimal> = BTreeMap::new();
    for record in &records {
        total += record.net_base;
        let name = record
            .security_id
            .as_ref()
            .and_then(|id| securities.iter().find(|s| &s.id == id))
            .map(|s| s.symbol.clone())
            .unwrap_or_else(|| "cash".to_string());
        *by_instrument.entry(name).or_default() += record.net_base;
    }

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "total": money(total),
        "payments": records.len(),
        "by_instrument": by_instrument
            .iter()
            .map(|(name, amount)| json!({ "instrument": name, "total": money(*amount) }))
            .collect::<Vec<_>>(),
    }))
}

pub(super) fn income_breakdown(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let split = analytics
        .income_by_taxonomy(&taxonomy.id, range.from, range.to, None)
        .map_err(tool)?;

    fn node_json(node: &sq_core::calc::IncomeNode) -> Value {
        json!({
            "name": node.label,
            "received": money(node.summary.net_base),
            "payments": node.summary.events,
            "share_percent": percent(node.weight),
            "children": node.children.iter().map(node_json).collect::<Vec<_>>(),
        })
    }

    Ok(json!({
        "tree": taxonomy.name,
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "total": money(split.total.net_base),
        "payments": split.total.events,
        "categories": split.nodes.iter().map(node_json).collect::<Vec<_>>(),
    }))
}

/// Without a limit the ledger of an active portfolio is thousands of rows; 200 is the cap so a
/// careless call costs a page of context, not a chapter.
pub(super) const TRANSACTIONS_CAP: usize = 200;

pub(super) fn transactions_list(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let mut transactions = analytics.transactions_until(Some(range.to)).map_err(tool)?;
    transactions.retain(|t| t.date >= range.from);
    transactions.sort_by_key(|t| std::cmp::Reverse(t.date));
    let total = transactions.len();
    transactions.truncate(limit_of(args).unwrap_or(TRANSACTIONS_CAP).min(TRANSACTIONS_CAP));

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "shown": transactions.len(),
        "matched": total,
        "rows": transactions
            .iter()
            .map(|t| json!({
                "date": t.date.to_string(),
                "kind": format!("{:?}", t.kind),
                "instrument": t
                    .security_id
                    .as_ref()
                    .and_then(|id| securities.iter().find(|s| &s.id == id))
                    .map(|s| s.symbol.as_str()),
                "quantity": quantity(t.quantity),
                "amount": money(t.amount),
                "currency": t.currency,
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The body of `payments_grid`: every dated figure of the period on one axis. Buckets carry
/// their own dates rather than a label — the name of a month belongs to the frontend.
pub(super) fn income_calendar(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let interval = match optional(args, "interval").as_deref() {
        Some("QUARTER") => sq_core::calc::PaymentPeriod::Quarter,
        Some("YEAR") => sq_core::calc::PaymentPeriod::Year,
        _ => sq_core::calc::PaymentPeriod::Month,
    };
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let grid = analytics.payments(range.from, range.to, interval).map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "interval": format!("{interval:?}").to_uppercase(),
        "base_currency": analytics.base_currency(),
        "earned_total": money(grid.earnings_total),
        "columns": grid
            .buckets
            .iter()
            .map(|bucket| json!({ "from": bucket.from.to_string(), "to": bucket.to.to_string() }))
            .collect::<Vec<_>>(),
        // One row per kind, in the same order as the columns above.
        "lines": grid
            .lines
            .iter()
            .map(|line| json!({
                "kind": format!("{:?}", line.line),
                "total": money(line.total),
                "amounts": line.amounts.iter().map(|a| money(*a)).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "earned_per_column": grid.earnings.iter().map(|a| money(*a)).collect::<Vec<_>>(),
        // Income only: a fee has no payer, and a disposal is a trade.
        "payers": grid
            .securities
            .iter()
            .take(PAYERS_CAP)
            .map(|row| json!({
                "instrument": row
                    .security_id
                    .as_ref()
                    .and_then(|id| securities.iter().find(|s| &s.id == id))
                    .map(|s| s.symbol.as_str())
                    // No instrument is the account's own interest, not a missing name.
                    .unwrap_or("cash"),
                "total": money(row.total),
                "amounts": row.amounts.iter().map(|a| money(*a)).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
    }))
}

/// How many payers one calendar names. The rows are largest first, so the tail is noise.
pub(super) const PAYERS_CAP: usize = 20;
