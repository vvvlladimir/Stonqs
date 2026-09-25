use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const SECURITIES_FIND: Tool = Tool {
    name: "securities_find",
    description: "Look an instrument up by ticker, name or ISIN. Use it to check an \
                  instrument exists and how it is spelled before quoting it back.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Part of a ticker, name or ISIN." }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("query", text(args, "query")),
    run: securities_find,
};

pub(super) const SECURITIES_EVENTS: Tool = Tool {
    name: "securities_events",
    description: "Dated events of one instrument: the dividends and splits its quote \
                  provider reported, and the user's own notes. Use it for \"when did it last \
                  pay\" or \"has it split\", never to total income — income_summary does that.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker in this portfolio." },
                "limit": { "type": ["integer", "null"], "description": "How many events, newest first. Null means 20." }
            },
            "required": ["symbol", "limit"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("instrument", text(args, "symbol")),
    run: securities_events,
};

pub(super) const SECURITY_SET_DATA_SOURCE: Tool = Tool {
    name: "security_set_data_source",
    description: "Change where an instrument's prices come from: the provider and the symbol \
                  it is quoted under there. Use it when quotes are missing or wrong for an \
                  instrument. Switching the symbol drops the price history stored under the \
                  old one.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker in this portfolio, as securities_find returned it." },
                "data_source": { "type": ["string", "null"], "description": "Quote provider id, e.g. \"yahoo\". Null means prices are entered by hand." },
                "data_symbol": { "type": ["string", "null"], "description": "The symbol that provider quotes it under, e.g. \"IWDA.L\". Null means use the ticker itself." }
            },
            "required": ["symbol", "data_source", "data_symbol"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("source".into(), nullable(args, "data_source"));
        params.insert("symbol".into(), nullable(args, "data_symbol"));
        params
    },
    run: security_set_data_source,
};

pub(super) const SECURITY_SET_NOTE: Tool = Tool {
    name: "security_set_note",
    description: "Replace the user's own note on an instrument. The note is theirs — read it \
                  first and only change it when asked to.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker in this portfolio." },
                "note": { "type": ["string", "null"], "description": "The new note. Null clears it." }
            },
            "required": ["symbol", "note"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("note".into(), nullable(args, "note"));
        params
    },
    run: security_set_note,
};

pub(super) const SECURITY_CREATE: Tool = Tool {
    name: "security_create",
    description: "Add an instrument to the portfolio's list so operations can be recorded \
                  against it. Call securities_find first — an instrument that already exists \
                  under another spelling must not be added twice.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The ticker to know it by, e.g. \"AAPL\"." },
                "name": { "type": "string", "description": "Its full name." },
                "currency": { "type": "string", "description": "The currency it trades in, as a three-letter code." },
                "kind": { "type": "string", "enum": ["STOCK", "ETF", "BOND", "FUND", "CRYPTO", "OTHER"], "description": "What kind of instrument it is." },
                "isin": { "type": ["string", "null"], "description": "Its ISIN, when known." },
                "data_source": { "type": ["string", "null"], "description": "Quote provider id, e.g. \"yahoo\". Null means prices are entered by hand." },
                "data_symbol": { "type": ["string", "null"], "description": "The symbol that provider quotes it under, when it differs from the ticker." }
            },
            "required": ["symbol", "name", "currency", "kind", "isin", "data_source", "data_symbol"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("symbol".into(), text(args, "symbol"));
        params.insert("name".into(), text(args, "name"));
        params.insert("currency".into(), text(args, "currency"));
        params.insert("kind".into(), text(args, "kind"));
        for key in ["isin", "data_source", "data_symbol"] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        params
    },
    run: security_create,
};

pub(super) const SECURITY_DELETE: Tool = Tool {
    name: "security_delete",
    description: "Remove an instrument from the portfolio's list, with its prices, alerts and \
                  classifications. An instrument that has ever been traded cannot be removed — \
                  its operations are the ledger, and they are deleted first if that is really \
                  what is wanted.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker." }
            },
            "required": ["symbol"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("instrument", text(args, "symbol")),
    run: security_delete,
};

pub(super) const SECURITY_SET_LISTING: Tool = Tool {
    name: "security_set_listing",
    description: "Point an instrument at a different venue's listing: its ticker there, the \
                  currency it trades in and the exchange. One ISIN trades in several places \
                  under different tickers, and the price history belongs to the ticker, so a \
                  real change throws the stored prices away and they are fetched again.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's current ticker in this portfolio." },
                "listing_symbol": { "type": "string", "description": "The ticker on the venue it should follow, e.g. \"EUNL.DE\"." },
                "currency": { "type": "string", "description": "The currency that listing trades in." },
                "mic": { "type": ["string", "null"], "description": "The venue's ISO 10383 code, e.g. \"XETR\". Null leaves the venue unnamed." }
            },
            "required": ["symbol", "listing_symbol", "currency", "mic"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("symbol".into(), text(args, "listing_symbol"));
        params.insert("currency".into(), text(args, "currency"));
        params.insert("mic".into(), nullable(args, "mic"));
        params
    },
    run: security_set_listing,
};

pub(super) const SECURITY_NOTE_ADD: Tool = Tool {
    name: "security_note_add",
    description: "Write a dated note against an instrument — why it was bought, what happened \
                  that day. It sits in that instrument's history beside the dividends and \
                  splits its provider reported, and moves no money.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker." },
                "date": { "type": ["string", "null"], "description": "YYYY-MM-DD. Null means today." },
                "note": { "type": "string", "description": "The note itself, in the user's words." }
            },
            "required": ["symbol", "date", "note"],
            "additionalProperties": false
        })
    },
    summary: |context, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("date".into(), date_of(context, args).to_string());
        params.insert("note".into(), text(args, "note"));
        params
    },
    run: security_note_add,
};

pub(super) const SECURITY_NOTE_DELETE: Tool = Tool {
    name: "security_note_delete",
    description: "Remove the user's own note from an instrument's history on one date. A \
                  dividend or split the provider reported is not a note and is never removed \
                  this way.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker." },
                "date": { "type": "string", "description": "The note's date, YYYY-MM-DD." }
            },
            "required": ["symbol", "date"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("date".into(), text(args, "date"));
        params
    },
    run: security_note_delete,
};

pub(super) fn securities_find(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let query = text(args, "query").to_lowercase();
    let matches: Vec<Value> = context
        .store
        .list_securities()
        .map_err(tool)?
        .iter()
        .filter(|s| {
            s.symbol.to_lowercase().contains(&query)
                || s.name.to_lowercase().contains(&query)
                || s.isin
                    .as_deref()
                    .is_some_and(|i| i.to_lowercase().contains(&query))
        })
        .take(20)
        .map(|s| {
            json!({
                "symbol": s.symbol,
                "name": s.name,
                "isin": s.isin,
                "currency": s.currency,
            })
        })
        .collect();

    Ok(json!({ "found": matches.len(), "instruments": matches }))
}

/// The body of `security_save`'s data-source half, and nothing more: the same validation and the
/// same consequence. `.claude/rules/money-and-fx.md` — changing the provider symbol throws away
/// the series stored under the old one, because the series belongs to the symbol.
pub(super) fn security_set_data_source(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = security_by_symbol(context, &text(args, "symbol"))?;
    let updated = sq_core::model::Security {
        data_source: optional(args, "data_source"),
        data_symbol: optional(args, "data_symbol"),
        ..existing.clone()
    };
    let resymbolled = updated.provider_symbol() != existing.provider_symbol();
    context.store.save_security(&updated).map_err(tool)?;
    if resymbolled {
        context.store.delete_quotes(&updated.id).map_err(tool)?;
    }
    (context.changed)("securities");

    Ok(json!({
        "instrument": updated.symbol,
        "data_source": updated.data_source,
        "data_symbol": updated.data_symbol,
        "quotes_cleared": resymbolled,
        "note": "New quotes are fetched in the background; they are not here yet.",
    }))
}

pub(super) fn security_set_note(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = security_by_symbol(context, &text(args, "symbol"))?;
    let updated = sq_core::model::Security {
        note: optional(args, "note"),
        ..existing
    };
    context.store.save_security(&updated).map_err(tool)?;
    (context.changed)("securities");
    Ok(json!({ "instrument": updated.symbol, "note": updated.note }))
}

/// How many events one instrument answers with by default. Enough to see a dividend rhythm,
/// short of reprinting a decade.
pub(super) const EVENTS_CAP: usize = 20;

pub(super) fn securities_events(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    let mut events = context.store.security_events_for(&security.id).map_err(tool)?;
    events.sort_by_key(|e| std::cmp::Reverse(e.date));
    let matched = events.len();
    events.truncate(limit_of(args).unwrap_or(EVENTS_CAP).min(EVENTS_CAP));

    Ok(json!({
        "instrument": security.symbol,
        "shown": events.len(),
        "matched": matched,
        "events": events
            .iter()
            .map(|event| json!({
                "date": event.date.to_string(),
                "kind": format!("{:?}", event.kind),
                "amount_per_share": event.amount.map(price),
                "currency": event.currency,
                "split_ratio": match (event.ratio_from, event.ratio_to) {
                    (Some(from), Some(to)) => Value::String(format!("{to}:{from}")),
                    _ => Value::Null,
                },
                "note": event.note,
                // Whose event it is: the provider reported it, or the user wrote it down.
                "reported_by": event.source,
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The body of `security_save` for a new instrument. The host fetches its quotes behind the
/// change notification, the same way the form does (`.claude/rules/money-and-fx.md`).
pub(super) fn security_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let symbol = text(args, "symbol").trim().to_uppercase();
    let name = text(args, "name").trim().to_string();
    if symbol.is_empty() || name.is_empty() {
        return Err(AiError::Tool("an instrument needs a ticker and a name".into()));
    }
    if security_by_symbol(context, &symbol).is_ok() {
        return Err(AiError::Tool(format!(
            "{symbol} is already in this portfolio; edit it instead of adding it again"
        )));
    }
    let kind: sq_core::model::SecurityKind = serde_json::from_value(json!(text(args, "kind")))
        .map_err(|_| AiError::Tool(format!("unknown instrument kind {}", text(args, "kind"))))?;

    let currency = sq_core::money::normalize_currency(&text(args, "currency"));
    let security = sq_core::model::Security {
        isin: optional(args, "isin").map(|i| i.to_uppercase()),
        data_source: optional(args, "data_source"),
        data_symbol: optional(args, "data_symbol"),
        ..sq_core::model::Security::new(&symbol, &name, &currency, kind)
    };
    context.store.save_security(&security).map_err(tool)?;
    (context.changed)("securities");

    Ok(json!({
        "instrument": security.symbol,
        "name": security.name,
        "currency": security.currency,
        "kind": format!("{:?}", security.kind),
        "isin": security.isin,
        "note": "Prices are fetched in the background; they are not here yet.",
    }))
}

pub(super) fn security_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    // The ledger holds the instrument: the database refuses the delete, and saying so plainly
    // is more use to the model than the constraint's own wording.
    let traded = context
        .store
        .transactions_for_accounts(&context.scope.portfolio.account_ids, None)
        .map_err(tool)?
        .iter()
        .filter(|t| t.security_id.as_deref() == Some(security.id.as_str()))
        .count();
    if traded > 0 {
        return Err(AiError::Tool(format!(
            "{} has {traded} operations in the ledger and cannot be removed while they exist",
            security.symbol
        )));
    }

    context.store.delete_security(&security.id).map_err(tool)?;
    (context.changed)("securities");
    Ok(json!({ "removed": security.symbol, "name": security.name }))
}

/// The body of `security_set_listing`. The series belongs to the symbol, so naming the venue of
/// a series already stored keeps it and only a different symbol drops it (ADR-0036).
pub(super) fn security_set_listing(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = security_by_symbol(context, &text(args, "symbol"))?;
    let symbol = text(args, "listing_symbol").trim().to_uppercase();
    if symbol.is_empty() {
        return Err(AiError::Tool("the listing needs a ticker".into()));
    }
    let currency = sq_core::money::normalize_currency(&text(args, "currency"));
    if currency.is_empty() {
        return Err(AiError::Tool("the listing needs a currency".into()));
    }

    let same_series = existing.provider_symbol() == symbol;
    let updated = sq_core::model::Security {
        symbol,
        currency,
        data_source: existing
            .data_source
            .clone()
            .or(context.quotes_source.map(str::to_string)),
        data_symbol: None,
        mic: optional(args, "mic").map(|m| m.trim().to_uppercase()),
        ..existing.clone()
    };
    context.store.save_security(&updated).map_err(tool)?;
    if !same_series {
        context.store.delete_quotes(&updated.id).map_err(tool)?;
    }
    (context.changed)("securities");

    Ok(json!({
        "instrument": updated.symbol,
        "was": existing.symbol,
        "currency": updated.currency,
        "venue": updated
            .mic
            .as_deref()
            .and_then(sq_core::market::mic::market_name),
        "quotes_cleared": !same_series,
    }))
}

pub(super) fn security_note_add(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    let note = text(args, "note");
    if note.trim().is_empty() {
        return Err(AiError::Tool("the note is empty".into()));
    }
    let date = date_of(context, args);

    let event = sq_core::model::SecurityEvent::note(&security.id, date, note.trim());
    context.store.save_security_event(&event).map_err(tool)?;
    (context.changed)("alerts");

    Ok(json!({
        "instrument": security.symbol,
        "date": date.to_string(),
        "note": event.note,
    }))
}

pub(super) fn security_note_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    let date =
        date_argument(args, "date")?.ok_or_else(|| AiError::Tool("say which day the note is on".into()))?;

    // Only what the user wrote: an event a provider reported is a fact about the instrument,
    // not a note, and deleting it would make the next refresh write it back anyway.
    let notes: Vec<sq_core::model::SecurityEvent> = context
        .store
        .security_events_for(&security.id)
        .map_err(tool)?
        .into_iter()
        .filter(|e| e.date == date && e.source.is_none())
        .collect();
    if notes.is_empty() {
        return Err(AiError::Tool(format!(
            "{} has no note of its own on {date}",
            security.symbol
        )));
    }
    for event in &notes {
        context.store.delete_security_event(&event.id).map_err(tool)?;
    }
    (context.changed)("alerts");

    Ok(json!({ "instrument": security.symbol, "date": date.to_string(), "removed": notes.len() }))
}
