//! The ledger tools that write: one operation created, changed or deleted, each asked every time.

use super::super::args::*;
use super::super::fmt::*;
use super::super::lookup::*;
use super::super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(in crate::ai::tools) const TRANSACTION_UPDATE: Tool = Tool {
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

pub(in crate::ai::tools) const TRANSACTION_DELETE: Tool = Tool {
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

pub(in crate::ai::tools) const TRANSACTION_CREATE: Tool = Tool {
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

/// `amount` of a share-moving kind is quantity × price, never typed; direction is the kind's.
fn transaction_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
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
        // A charge billed in another currency is not something the model is asked for: the
        // catalogue takes one figure per charge, in the operation's own currency.
        fee_currency: None,
        tax_currency: None,
        // The rate of the day is fixed on the transaction when there is one to fix; left absent,
        // valuation reads the rate on file for that date (`.claude/rules/money-and-fx.md`).
        fx_rate_to_base: None,
        // A linked pair is two rows written together; this writes one.
        link_id: None,
        // Only a broker file names a row; one written here is the user's own.
        external_id: None,
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

/// Changing either leg recomputes the amount of a share-moving row.
fn transaction_update(context: &ToolContext, args: &Value) -> AiResult<Value> {
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

fn transaction_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
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
