use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const ALERTS_LIST: Tool = Tool {
    name: "alerts_list",
    description: "The price and date rules the user set, with where the price stands against \
                  each level and how far it has to move. A rule is about an instrument, so \
                  the account lens does not apply.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": ["string", "null"], "description": "Only this instrument's rules. Null means every rule." }
            },
            "required": ["symbol"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("instrument", or_all(args, "symbol")),
    run: alerts_list,
};

pub(super) const ALERTS_CROSSINGS: Tool = Tool {
    name: "alerts_crossings",
    description: "The log of levels actually crossed, newest first: which rule, which day, \
                  which way, and the close that crossed it.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "limit": { "type": ["integer", "null"], "description": "How many lines, newest first. Null means 50." }
            },
            "required": ["limit"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("limit", limit_of(args).unwrap_or(CROSSINGS_CAP).to_string()),
    run: alerts_crossings,
};

pub(super) const ALERT_CREATE: Tool = Tool {
    name: "alert_create",
    description: "Watch an instrument: a price level whose crossings are logged, or a date to \
                  be reminded of. A price rule starts watching today — it never reports a \
                  level crossed before it existed.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker in this portfolio." },
                "kind": { "type": "string", "enum": ["PRICE", "DATE"], "description": "A price level, or a day to be reminded of." },
                "price": { "type": ["string", "null"], "description": "The level, in the instrument's own quote currency. Required for a price rule." },
                "direction": { "type": ["string", "null"], "enum": ["UP", "DOWN", "BOTH", null], "description": "Which crossings to log. Null means both." },
                "date": { "type": ["string", "null"], "description": "The day, YYYY-MM-DD. Required for a date rule." },
                "note": { "type": ["string", "null"], "description": "Why it matters. Null for none." }
            },
            "required": ["symbol", "kind", "price", "direction", "date", "note"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("kind".into(), text(args, "kind"));
        let level = nullable(args, "price");
        if !level.is_empty() {
            params.insert("price".into(), level);
            params.insert("direction".into(), or_both(args));
        }
        let date = nullable(args, "date");
        if !date.is_empty() {
            params.insert("date".into(), date);
        }
        params
    },
    run: alert_create,
};

pub(super) const ALERT_DELETE: Tool = Tool {
    name: "alert_delete",
    description: "Remove a price or date rule from an instrument. The crossings it already \
                  logged stay in the log — they happened.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker." },
                "price": { "type": ["string", "null"], "description": "The level of the price rule to remove. Null when removing a date rule." },
                "date": { "type": ["string", "null"], "description": "The day of the date rule to remove, YYYY-MM-DD. Null when removing a price rule." }
            },
            "required": ["symbol", "price", "date"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("level".into(), nullable(args, "price"));
        params.insert("date".into(), nullable(args, "date"));
        params
    },
    run: alert_delete,
};

pub(super) fn alerts_list(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let only = match optional(args, "symbol") {
        Some(symbol) => Some(security_by_symbol(context, &symbol)?),
        None => None,
    };
    let alerts = match &only {
        Some(security) => context.store.alerts_for_security(&security.id).map_err(tool)?,
        None => context.store.list_alerts().map_err(tool)?,
    };
    let securities = context.store.list_securities().map_err(tool)?;

    let mut rows = Vec::with_capacity(alerts.len());
    for alert in &alerts {
        let prices = context
            .store
            .quote_series(&alert.security_id, context.today)
            .map_err(tool)?;
        // A rule waiting for a quote or a rate is reported as waiting, never as "far from the
        // level" — a missing price is not a distance (`.claude/rules/money-and-fx.md`).
        let status = sq_core::calc::alert_status(alert, &prices, context.store, context.today).ok();
        let security = securities.iter().find(|s| s.id == alert.security_id);

        rows.push(json!({
            "instrument": security.map(|s| s.symbol.as_str()).unwrap_or("?"),
            "kind": format!("{:?}", alert.kind),
            "level": alert.price.map(price),
            "currency": alert.currency,
            "date": alert.date.map(|d| d.to_string()),
            "direction": format!("{:?}", alert.direction),
            "note": alert.note,
            "price": status.as_ref().and_then(|s| s.price).map(price),
            "price_date": status.as_ref().and_then(|s| s.price_date).map(|d| d.to_string()),
            "side": status.as_ref().and_then(|s| s.side).map(|side| format!("{side:?}")),
            "distance_percent": status.as_ref().and_then(|s| s.distance).map(percent),
            "waiting_for_data": status.is_none(),
        }));
    }

    Ok(json!({ "rules": rows.len(), "alerts": rows }))
}

/// The log is read newest first and capped, like the ledger: a portfolio watched for years has
/// more crossings than an answer can use.
pub(super) const CROSSINGS_CAP: usize = 50;

pub(super) fn alerts_crossings(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let limit = limit_of(args).unwrap_or(CROSSINGS_CAP).min(CROSSINGS_CAP);
    let crossings = context.store.alert_crossings(None, limit).map_err(tool)?;
    let alerts = context.store.list_alerts().map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let rows: Vec<Value> = crossings
        .iter()
        .filter_map(|crossing| {
            let alert = alerts.iter().find(|a| a.id == crossing.alert_id)?;
            let security = securities.iter().find(|s| s.id == alert.security_id);
            Some(json!({
                "date": crossing.date.to_string(),
                "instrument": security.map(|s| s.symbol.as_str()).unwrap_or("?"),
                "direction": format!("{:?}", crossing.direction),
                "level": crossing.level.map(price),
                "price": crossing.price.map(price),
                "currency": crossing.currency,
                "note": alert.note,
            }))
        })
        .collect();

    Ok(json!({ "shown": rows.len(), "crossings": rows }))
}

/// The body of `alert_save` for a new rule: the same two constructors, so a rule the assistant
/// wrote behaves exactly like one typed into the dialog — including starting its watch today,
/// which is what keeps a level crossed last month from arriving as news (ADR-0034).
pub(super) fn alert_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;

    let base = match text(args, "kind").as_str() {
        "DATE" => {
            let date = date_argument(args, "date")?
                .ok_or_else(|| AiError::Tool("a date rule needs a date".into()))?;
            sq_core::model::SecurityAlert::date_reached(&security.id, date, context.today)
        }
        _ => {
            let level = decimal_argument(args, "price")?
                .ok_or_else(|| AiError::Tool("a price rule needs a level".into()))?;
            // The level is read in the currency the instrument is quoted in, which is not
            // always the currency recorded for it (`.claude/rules/money-and-fx.md`).
            let currency = context
                .store
                .latest_quote_currency(&security.id)
                .map_err(tool)?
                .unwrap_or_else(|| security.currency.clone());
            let direction = match optional(args, "direction").as_deref() {
                Some("UP") => sq_core::model::AlertDirection::Up,
                Some("DOWN") => sq_core::model::AlertDirection::Down,
                _ => sq_core::model::AlertDirection::Both,
            };
            sq_core::model::SecurityAlert::price(&security.id, level, &currency, context.today)
                .with_direction(direction)
        }
    };

    let alert = sq_core::model::SecurityAlert {
        note: optional(args, "note"),
        ..base
    };
    context.store.save_alert(&alert).map_err(tool)?;
    (context.changed)("alerts");

    Ok(json!({
        "instrument": security.symbol,
        "kind": format!("{:?}", alert.kind),
        "price": alert.price.map(price),
        "currency": alert.currency,
        "date": alert.date.map(|d| d.to_string()),
        "direction": format!("{:?}", alert.direction),
    }))
}

pub(super) fn alert_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    let level = decimal_argument(args, "price")?;
    let date = date_argument(args, "date")?;
    if level.is_none() && date.is_none() {
        return Err(AiError::Tool(
            "say which rule: a level for a price rule, or a date for a date rule".into(),
        ));
    }

    let matching: Vec<sq_core::model::SecurityAlert> = context
        .store
        .alerts_for_security(&security.id)
        .map_err(tool)?
        .into_iter()
        .filter(|alert| level.is_none_or(|l| alert.price == Some(l)))
        .filter(|alert| date.is_none_or(|d| alert.date == Some(d)))
        .collect();
    if matching.is_empty() {
        return Err(AiError::Tool(format!("{} has no such rule", security.symbol)));
    }
    for alert in &matching {
        context.store.delete_alert(&alert.id).map_err(tool)?;
    }
    (context.changed)("alerts");

    Ok(json!({
        "instrument": security.symbol,
        "removed": matching.len(),
        // The log keeps what already crossed: deleting the rule does not undo the day it fired.
        "crossings_kept": true,
    }))
}
