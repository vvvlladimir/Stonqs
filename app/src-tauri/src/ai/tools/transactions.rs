use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

pub(super) const TRANSACTION_UPDATE: Tool = Tool {
    name: "transaction_update",
    description: "Correct one operation already in the ledger. The first four arguments say \
                  which row is meant and must match exactly one — call transactions_list first. \
                  Every \"new_\" argument left null keeps what the row has.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "date": { "type": "string", "description": "The row's current date, YYYY-MM-DD." },
                "kind": { "type": ["string", "null"], "enum": kind_enum_with_null(), "description": "The row's operation, to tell two rows of one day apart." },
                "symbol": { "type": ["string", "null"], "description": "The row's instrument, by ticker." },
                "account": { "type": ["string", "null"], "description": "The row's account, by name." },
                "new_date": { "type": ["string", "null"], "description": "Move the operation to this date." },
                "new_quantity": { "type": ["string", "null"], "description": "Shares, always positive." },
                "new_price": { "type": ["string", "null"], "description": "Price per share, in the row's currency." },
                "new_amount": { "type": ["string", "null"], "description": "Cash amount, for a kind that moves money rather than shares." },
                "new_fees": { "type": ["string", "null"], "description": "Commission on the operation." },
                "new_taxes": { "type": ["string", "null"], "description": "Tax withheld on the operation." },
                "new_note": { "type": ["string", "null"], "description": "The note on the row." }
            },
            "required": ["date", "kind", "symbol", "account", "new_date", "new_quantity", "new_price", "new_amount", "new_fees", "new_taxes", "new_note"],
            "additionalProperties": false
        })
    },
    summary: match_summary,
    run: transaction_update,
};

pub(super) const TRANSACTION_DELETE: Tool = Tool {
    name: "transaction_delete",
    description: "Remove one operation from the ledger. The arguments must match exactly one \
                  row — call transactions_list first and narrow until they do. Everything \
                  computed from that row disappears with it.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "date": { "type": "string", "description": "The row's date, YYYY-MM-DD." },
                "kind": { "type": ["string", "null"], "enum": kind_enum_with_null(), "description": "The row's operation." },
                "symbol": { "type": ["string", "null"], "description": "The row's instrument, by ticker." },
                "account": { "type": ["string", "null"], "description": "The row's account, by name." }
            },
            "required": ["date", "kind", "symbol", "account"],
            "additionalProperties": false
        })
    },
    summary: match_summary,
    run: transaction_delete,
};

pub(super) const TRANSACTION_CREATE: Tool = Tool {
    name: "transaction_create",
    description: "Record one operation in the ledger: a purchase, a sale, a dividend, a \
                  deposit, a fee and so on. Everything else in the app is computed from these \
                  rows, so record what actually happened at the broker, never a rounded \
                  version of it. Read accounts_list first: the money side of a trade settles \
                  on a deposit account.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": TRANSACTION_KINDS, "description": "What happened." },
                "account": { "type": "string", "description": "The account's name, as accounts_list returned it. A purchase belongs on a securities account." },
                "date": { "type": ["string", "null"], "description": "YYYY-MM-DD. Null means today." },
                "symbol": { "type": ["string", "null"], "description": "The instrument's ticker, for anything involving one. Null for a pure cash operation." },
                "quantity": { "type": ["string", "null"], "description": "Shares, for a kind that moves them. Always positive: the kind carries the direction." },
                "price": { "type": ["string", "null"], "description": "Price per share in the transaction's currency." },
                "amount": { "type": ["string", "null"], "description": "The cash amount, for a kind that moves money rather than shares. Always positive." },
                "fees": { "type": ["string", "null"], "description": "Commission charged on this operation. Null for none." },
                "taxes": { "type": ["string", "null"], "description": "Tax withheld on this operation. Null for none." },
                "currency": { "type": ["string", "null"], "description": "The operation's currency. Null means the account's own." },
                "note": { "type": ["string", "null"], "description": "The user's note on the row." }
            },
            "required": ["kind", "account", "date", "symbol", "quantity", "price", "amount", "fees", "taxes", "currency", "note"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = Params::new();
        params.insert("operation".into(), text(args, "kind"));
        params.insert("account".into(), text(args, "account"));
        params.insert("date".into(), date_of(context, args).to_string());
        for key in [
            "symbol", "quantity", "price", "amount", "fees", "taxes", "currency",
        ] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        params
    },
    run: transaction_create,
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

/// The body of `transaction_save` for a new row, with the same rule about what `amount` means:
/// for a kind that moves shares it is quantity times price, and typing one directly would let a
/// row exist whose amount and its own legs disagree. The direction is the kind's, never the
/// sign of a number — "sell minus five shares" cannot happen.
pub(super) fn transaction_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let kind: sq_core::model::TransactionKind = serde_json::from_value(json!(text(args, "kind")))
        .map_err(|_| AiError::Tool(format!("unknown operation {}", text(args, "kind"))))?;
    let account = account_by_name(context, &text(args, "account"))?;
    let security = match optional(args, "symbol") {
        Some(symbol) => Some(security_by_symbol(context, &symbol)?),
        None => None,
    };

    let shares = decimal_argument(args, "quantity")?.unwrap_or_default();
    let unit_price = decimal_argument(args, "price")?.unwrap_or_default();
    let amount = if kind.affects_quantity() {
        shares * unit_price
    } else {
        decimal_argument(args, "amount")?.unwrap_or_default()
    };
    if kind.affects_quantity() && security.is_none() {
        return Err(AiError::Tool(
            "this operation moves shares, so it needs an instrument".into(),
        ));
    }

    let transaction = sq_core::model::Transaction {
        id: sq_core::model::new_id(),
        account_id: account.id.clone(),
        security_id: security.as_ref().map(|s| s.id.clone()),
        kind,
        date: date_of(context, args),
        quantity: shares,
        price: unit_price,
        amount,
        fees: decimal_argument(args, "fees")?.unwrap_or_default(),
        taxes: decimal_argument(args, "taxes")?.unwrap_or_default(),
        currency: sq_core::money::normalize_currency(
            &optional(args, "currency").unwrap_or_else(|| account.currency.clone()),
        ),
        // The rate of the day is fixed on the transaction when there is one to fix; left absent,
        // valuation reads the rate on file for that date (`.claude/rules/money-and-fx.md`).
        fx_rate_to_base: None,
        // A linked pair is two rows written together; this writes one.
        link_id: None,
        note: optional(args, "note"),
        // Nothing the user writes is a lens's rewrite of something else.
        scoped_from: None,
    };
    context.store.save_transaction(&transaction).map_err(tool)?;
    // A new trade can bring the first quote of an instrument, or a currency pair nothing has a
    // rate for; the host fetches both behind this.
    (context.changed)("transactions");

    Ok(json!({
        "recorded": format!("{:?}", transaction.kind),
        "date": transaction.date.to_string(),
        "account": account.name,
        "instrument": security.map(|s| s.symbol),
        "quantity": quantity(transaction.quantity),
        "price": price(transaction.price),
        "amount": money(transaction.amount),
        "currency": transaction.currency,
    }))
}

/// The operations a row may be, plus the null a `strict` schema spells an optional argument as.
fn kind_enum_with_null() -> Value {
    let mut kinds: Vec<Value> = TRANSACTION_KINDS.iter().map(|k| json!(k)).collect();
    kinds.push(Value::Null);
    Value::Array(kinds)
}

/// What the card shows for a row being changed: the values that pick it out, so the user reads
/// which operation is meant before allowing anything to happen to it.
fn match_summary(_context: &ToolContext, args: &Value) -> Params {
    let mut params = Params::new();
    params.insert("date".into(), text(args, "date"));
    for key in ["kind", "symbol", "account"] {
        let value = nullable(args, key);
        if !value.is_empty() {
            params.insert(key.into(), value);
        }
    }
    for key in [
        "new_date",
        "new_quantity",
        "new_price",
        "new_amount",
        "new_fees",
        "new_taxes",
        "new_note",
    ] {
        let value = nullable(args, key);
        if !value.is_empty() {
            params.insert(key.into(), value);
        }
    }
    params
}

/// The one row the arguments name. Editing the wrong operation is worse than editing none, so
/// two matches are refused with the candidates spelled out rather than resolved by a guess.
fn matching_transaction(context: &ToolContext, args: &Value) -> AiResult<sq_core::model::Transaction> {
    let date = date_argument(args, "date")?
        .ok_or_else(|| AiError::Tool("say which day the operation is on".into()))?;
    let account = match optional(args, "account") {
        Some(name) => Some(account_by_name(context, &name)?),
        None => None,
    };
    let security = match optional(args, "symbol") {
        Some(symbol) => Some(security_by_symbol(context, &symbol)?),
        None => None,
    };
    let kind = match optional(args, "kind") {
        Some(raw) => Some(
            serde_json::from_value::<sq_core::model::TransactionKind>(json!(raw))
                .map_err(|_| AiError::Tool(format!("unknown operation {raw}")))?,
        ),
        None => None,
    };

    // The whole portfolio, not the lens: a row exists whether or not the picker is looking at it.
    let mut found: Vec<sq_core::model::Transaction> = context
        .store
        .transactions_for_accounts(&context.scope.portfolio.account_ids, None)
        .map_err(tool)?
        .into_iter()
        .filter(|t| t.date == date)
        .filter(|t| account.as_ref().is_none_or(|a| t.account_id == a.id))
        .filter(|t| {
            security
                .as_ref()
                .is_none_or(|s| t.security_id.as_deref() == Some(s.id.as_str()))
        })
        .filter(|t| kind.is_none_or(|k| t.kind == k))
        .collect();

    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(AiError::Tool(format!("no operation on {date} matches that"))),
        n => Err(AiError::Tool(format!(
            "{n} operations on {date} match that: {}. Say which one by naming the operation, \
             the instrument or the account.",
            found
                .iter()
                .map(|t| format!("{:?} {}", t.kind, t.amount))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
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

/// The body of `transaction_save` for a row that already exists. The amount of a share-moving
/// row stays its own legs multiplied, so changing either recomputes it rather than leaving a
/// row whose amount and legs disagree.
pub(super) fn transaction_update(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = matching_transaction(context, args)?;

    let shares = decimal_argument(args, "new_quantity")?.unwrap_or(existing.quantity);
    let unit_price = decimal_argument(args, "new_price")?.unwrap_or(existing.price);
    let amount = if existing.kind.affects_quantity() {
        shares * unit_price
    } else {
        decimal_argument(args, "new_amount")?.unwrap_or(existing.amount)
    };

    let updated = sq_core::model::Transaction {
        date: date_argument(args, "new_date")?.unwrap_or(existing.date),
        quantity: shares,
        price: unit_price,
        amount,
        fees: decimal_argument(args, "new_fees")?.unwrap_or(existing.fees),
        taxes: decimal_argument(args, "new_taxes")?.unwrap_or(existing.taxes),
        note: optional(args, "new_note").or(existing.note.clone()),
        ..existing
    };
    context.store.save_transaction(&updated).map_err(tool)?;
    (context.changed)("transactions");

    Ok(json!({
        "changed": format!("{:?}", updated.kind),
        "date": updated.date.to_string(),
        "quantity": quantity(updated.quantity),
        "price": price(updated.price),
        "amount": money(updated.amount),
        "currency": updated.currency,
        "fees": money(updated.fees),
        "taxes": money(updated.taxes),
    }))
}

pub(super) fn transaction_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = matching_transaction(context, args)?;
    let securities = context.store.list_securities().map_err(tool)?;
    let symbol = existing
        .security_id
        .as_ref()
        .and_then(|id| securities.iter().find(|s| &s.id == id))
        .map(|s| s.symbol.clone());

    context.store.delete_transaction(&existing.id).map_err(tool)?;
    (context.changed)("transactions");

    Ok(json!({
        "removed": format!("{:?}", existing.kind),
        "date": existing.date.to_string(),
        "instrument": symbol,
        "amount": money(existing.amount),
        "currency": existing.currency,
    }))
}
