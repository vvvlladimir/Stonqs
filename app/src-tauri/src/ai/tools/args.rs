//! Reading the model's arguments. A tool body never touches the raw JSON: it asks here
//! for a string, a date, a decimal or a period, and gets back something already checked.

use super::{AiError, AiResult, Params, ToolContext, tool};
use chrono::NaiveDate;
use serde_json::{Value, json};
use sq_core::calc::PeriodPreset;
use sq_core::market::DateRange;

pub(super) fn no_arguments() -> Value {
    json!({ "type": "object", "properties": {}, "required": [], "additionalProperties": false })
}

/// The shipped axis only: a user's own period is theirs to name and is not offered to the model,
/// which would have no way to know what it means.
pub(super) const PERIODS: &[(&str, PeriodPreset)] = &[
    ("ONE_MONTH", PeriodPreset::OneMonth),
    ("THREE_MONTHS", PeriodPreset::ThreeMonths),
    ("YTD", PeriodPreset::Ytd),
    ("ONE_YEAR", PeriodPreset::OneYear),
    ("THREE_YEARS", PeriodPreset::ThreeYears),
    ("FIVE_YEARS", PeriodPreset::FiveYears),
    ("SINCE_INCEPTION", PeriodPreset::SinceInception),
];

pub(super) fn period_property() -> Value {
    json!({
        "type": "string",
        "enum": PERIODS.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        "description": "Which window to report over."
    })
}

pub(super) fn period_argument() -> Value {
    json!({
        "type": "object",
        "properties": { "period": period_property() },
        "required": ["period"],
        "additionalProperties": false
    })
}

pub(super) fn period_summary(context: &ToolContext, args: &Value) -> Params {
    let mut params = Params::new();
    match resolve_period(context, args) {
        Ok(range) => {
            params.insert("from".into(), range.from.to_string());
            params.insert("to".into(), range.to.to_string());
        }
        Err(_) => {
            params.insert("period".into(), text(args, "period"));
        }
    }
    params
}

/// The period is resolved by the core against the portfolio's own inception, exactly as
/// `period_ranges` does it for the UI — the model names a window, never a pair of dates.
pub(super) fn resolve_period(context: &ToolContext, args: &Value) -> AiResult<DateRange> {
    let id = text(args, "period");
    let preset = PERIODS
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, preset)| *preset)
        .ok_or_else(|| AiError::Tool(format!("unknown period {id}")))?;
    let inception = context
        .scope
        .analytics(context.store)
        .map_err(tool)?
        .inception()
        .map_err(tool)?;
    preset.range(context.today, inception).map_err(tool)
}

pub(super) fn text(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn limit_of(args: &Value) -> Option<usize> {
    args.get("limit").and_then(Value::as_u64).map(|n| n as usize)
}

/// A `strict` schema spells an optional argument as a nullable one, so "absent" arrives as JSON
/// null rather than a missing key — both read back as nothing here.
pub(super) fn nullable(args: &Value, key: &str) -> String {
    match args.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        _ => String::new(),
    }
}

pub(super) fn optional(args: &Value, key: &str) -> Option<String> {
    let value = nullable(args, key);
    (!value.is_empty()).then_some(value)
}

/// A nullable argument as a card value: "all" rather than an empty cell, because the card says
/// what the call covers and "every instrument" is the wider request of the two.
pub(super) fn or_all(args: &Value, key: &str) -> String {
    match optional(args, key) {
        Some(value) => value,
        None => "all".to_string(),
    }
}

/// The operations the editor offers, spelled as they cross everywhere else. Deliberately not
/// every variant the ledger knows: the paired transfer kinds are two rows linked to each other,
/// and half of such a pair written on its own reads to `calc` as money leaving the portfolio.
pub(super) const TRANSACTION_KINDS: &[&str] = &[
    "BUY",
    "SELL",
    "DIVIDEND",
    "DEPOSIT",
    "WITHDRAWAL",
    "INTEREST",
    "INTEREST_CHARGE",
    "CASHBACK",
    "REWARD",
    "FEE",
    "FEE_REFUND",
    "TAX",
    "TAX_REFUND",
    "DELIVERY_INBOUND",
    "DELIVERY_OUTBOUND",
];

pub(super) fn or_both(args: &Value) -> String {
    optional(args, "direction").unwrap_or_else(|| "BOTH".to_string())
}

pub(super) fn symbols_of(args: &Value) -> Vec<String> {
    names_of(args, "symbols")
}

/// A list-of-strings argument — tickers, account names — as the model wrote it.
pub(super) fn names_of(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// A date argument, parsed the way the host parses one on the wire; absent means absent, and a
/// malformed one is the model's mistake to hear about rather than a silent "today".
pub(super) fn date_argument(args: &Value, key: &str) -> AiResult<Option<NaiveDate>> {
    match optional(args, key) {
        None => Ok(None),
        Some(raw) => NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .map(Some)
            .map_err(|_| AiError::Tool(format!("{raw} is not a date of the form YYYY-MM-DD"))),
    }
}

/// What a write is dated when the model does not say. Today, like the editor's own form.
pub(super) fn date_of(context: &ToolContext, args: &Value) -> NaiveDate {
    date_argument(args, "date")
        .ok()
        .flatten()
        .unwrap_or(context.today)
}

/// A number as the model wrote it. The comma is accepted for the same reason the editor accepts
/// it: the user dictated "10,5" and the model passed it on.
pub(super) fn decimal_argument(args: &Value, key: &str) -> AiResult<Option<rust_decimal::Decimal>> {
    use std::str::FromStr;
    match optional(args, key) {
        None => Ok(None),
        Some(raw) => rust_decimal::Decimal::from_str(raw.trim().replace(',', ".").as_str())
            .map(Some)
            .map_err(|_| AiError::Tool(format!("{key} is not a number: {raw}"))),
    }
}

pub(super) fn one(key: &str, value: String) -> Params {
    let mut params = Params::new();
    params.insert(key.to_string(), value);
    params
}
