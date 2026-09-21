use super::args::*;
use super::lookup::*;
use super::market::instrument_move_json;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const WATCHLISTS_LIST: Tool = Tool {
    name: "watchlists_list",
    description: "The instrument lists the user keeps, with how many instruments each holds. \
                  Call this first to learn a list's name before asking for its rows.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: watchlists_list,
};

pub(super) const WATCHLIST_ROWS: Tool = Tool {
    name: "watchlist_rows",
    description: "One watchlist's instruments with each one's own price move over a period. \
                  A watched instrument need not be held: these are prices, not positions.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "list": { "type": "string", "description": "The list's name, exactly as watchlists_list returned it." },
                "period": period_property()
            },
            "required": ["list", "period"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = period_summary(context, args);
        params.insert("list".into(), text(args, "list"));
        params
    },
    run: watchlist_rows,
};

pub(super) const WATCHLIST_SET: Tool = Tool {
    name: "watchlist_set",
    description: "Create a watchlist or replace what is on one. The instruments given are \
                  the whole list afterwards, in that order — read watchlist_rows first and \
                  send the existing tickers back along with the new ones, or the rest are \
                  dropped.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "list": { "type": "string", "description": "The list's name. A name that does not exist yet creates the list." },
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Every instrument the list should hold afterwards, by ticker, in the order they should appear."
                }
            },
            "required": ["list", "symbols"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("list".into(), text(args, "list"));
        params.insert("instruments".into(), symbols_of(args).join(", "));
        params
    },
    run: watchlist_set,
};

pub(super) const WATCHLIST_DELETE: Tool = Tool {
    name: "watchlist_delete",
    description: "Delete a watchlist. The instruments on it stay in the portfolio, and any \
                  price alerts on them keep running — only the list goes.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "list": { "type": "string", "description": "The list's name, as watchlists_list returned it." }
            },
            "required": ["list"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("list", text(args, "list")),
    run: watchlist_delete,
};

pub(super) fn watchlists_list(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let lists: Vec<Value> = context
        .store
        .list_watchlists()
        .map_err(tool)?
        .iter()
        .map(|list| json!({ "name": list.name, "instruments": list.security_ids.len() }))
        .collect();
    Ok(json!({ "lists": lists }))
}

pub(super) fn watchlist_rows(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let list = watchlist_by_name(context, &text(args, "list"))?;
    let range = resolve_period(context, args)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let mut rows = Vec::new();
    for security in list
        .security_ids
        .iter()
        .filter_map(|id| securities.iter().find(|s| &s.id == id))
    {
        rows.push(json!({
            "instrument": security.symbol,
            "name": security.name,
            "quote": instrument_move_json(context, security, range)?,
        }));
    }

    Ok(json!({
        "list": list.name,
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "rows": rows,
    }))
}

/// A save replaces a list's instruments, which is what the storage does and what the screen does
/// (ADR-0035) — so the tool says so rather than quietly adding to what is there.
pub(super) fn watchlist_set(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let name = text(args, "list");
    if name.trim().is_empty() {
        return Err(AiError::Tool("a watchlist needs a name".into()));
    }
    let existing = context
        .store
        .list_watchlists()
        .map_err(tool)?
        .into_iter()
        .find(|l| l.name.eq_ignore_ascii_case(&name));

    let mut security_ids = Vec::new();
    let mut symbols = Vec::new();
    for symbol in symbols_of(args) {
        let security = security_by_symbol(context, &symbol)?;
        if !security_ids.contains(&security.id) {
            security_ids.push(security.id);
            symbols.push(security.symbol);
        }
    }

    let list = sq_core::model::Watchlist {
        id: existing
            .as_ref()
            .map(|l| l.id.clone())
            .unwrap_or_else(sq_core::model::new_id),
        name: name.trim().to_string(),
        security_ids,
    };
    context.store.save_watchlist(&list).map_err(tool)?;
    (context.changed)("watchlists");

    Ok(json!({
        "list": list.name,
        "created": existing.is_none(),
        "instruments": symbols,
    }))
}

pub(super) fn watchlist_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let list = watchlist_by_name(context, &text(args, "list"))?;
    context.store.delete_watchlist(&list.id).map_err(tool)?;
    (context.changed)("watchlists");
    Ok(json!({ "removed": list.name, "instruments": list.security_ids.len() }))
}
