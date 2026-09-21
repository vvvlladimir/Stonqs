use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiResult, Tool, ToolContext, tool};
use serde_json::{Value, json};
use sq_core::market::DateRange;

pub(super) const MARKET_QUOTES: Tool = Tool {
    name: "market_quotes",
    description: "How one instrument's own price moved over a period: latest close, the day's \
                  change, the move since the period started, and the range it traded in. \
                  The instrument's own quote currency, not the portfolio's base currency, \
                  and nothing to do with how much of it the user holds.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "symbol": { "type": "string", "description": "The instrument's ticker in this portfolio." },
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
    run: market_quotes,
};

pub(super) fn market_quotes(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = resolve_period(context, args)?;
    let security = security_by_symbol(context, &text(args, "symbol"))?;
    Ok(json!({
        "instrument": security.symbol,
        "name": security.name,
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "quote": instrument_move_json(context, &security, range)?,
    }))
}

/// The instrument's own quotes in its own currency — never converted, never weighted by what is
/// held. `calc::instrument_move` is the same reading the watchlist shows (ADR-0035).
pub(super) fn instrument_move_json(
    context: &ToolContext,
    security: &sq_core::model::Security,
    range: DateRange,
) -> AiResult<Value> {
    let prices = context.store.quote_series(&security.id, range.to).map_err(tool)?;
    let events = context.store.security_events_for(&security.id).map_err(tool)?;
    let moved = sq_core::calc::instrument_move(&prices, &events, range.from, range.to);

    Ok(json!({
        "currency": moved.currency,
        "price": moved.price.map(price),
        "price_date": moved.price_date.map(|d| d.to_string()),
        "day_change_percent": moved.day_change.map(percent),
        "period_return_percent": moved.period_return.map(percent),
        "period_start": moved.period_start.map(|d| d.to_string()),
        "low": moved.low.map(price),
        "high": moved.high.map(price),
    }))
}
