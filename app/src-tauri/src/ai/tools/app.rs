use super::args::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use crate::ai::guide;
use serde_json::{Value, json};

pub(super) const APP_PERIODS: Tool = Tool {
    name: "app_periods",
    description: "The period ids this portfolio can answer for, with the dates each one \
                  resolves to. Reads no portfolio data. Call it when unsure which period to \
                  pass to another tool.",
    access: Access::Free,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: app_periods,
};

pub(super) const APP_REFERENCE: Tool = Tool {
    name: "app_reference",
    description: "How one domain concept works in this app: what a figure means, why two \
                  figures that look alike differ, what a setting changes. Reads no portfolio \
                  data. Call it before explaining a concept rather than answering from \
                  general knowledge — this app's definitions are the ones the screens use.",
    access: Access::Free,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "enum": guide::topics(), "description": "The topic to read." }
            },
            "required": ["topic"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("topic", text(args, "topic")),
    run: app_reference,
};

pub(super) const APP_USER_GUIDE: Tool = Tool {
    name: "app_user_guide",
    description: "How one screen of the app works: what it shows, what its controls change, \
                  what confuses people about it. Reads no portfolio data. Every screen has a \
                  guide; if one ever answers with nothing, say you do not know how that part \
                  works rather than guessing.",
    access: Access::Free,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "screen": { "type": "string", "enum": guide::SCREEN_IDS, "description": "The screen to read about." }
            },
            "required": ["screen"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("screen", text(args, "screen")),
    run: app_user_guide,
};

pub(super) fn app_periods(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let inception = context
        .scope
        .analytics(context.store)
        .map_err(tool)?
        .inception()
        .map_err(tool)?;

    // A window the history cannot answer is left out rather than offered broken — the same rule
    // `period_ranges` applies to the strip the user sees.
    let periods: Vec<Value> = PERIODS
        .iter()
        .filter_map(|(id, preset)| {
            let range = preset.range(context.today, inception).ok()?;
            Some(json!({ "period": id, "from": range.from.to_string(), "to": range.to.to_string() }))
        })
        .collect();

    Ok(json!({ "today": context.today.to_string(), "periods": periods }))
}

/// Documentation, not data — but handed over through the same fenced result as everything else.
/// The fence's promise is that the model does not take orders from a tool result, and text the
/// repository wrote is no reason to make an exception to it.
pub(super) fn app_reference(_context: &ToolContext, args: &Value) -> AiResult<Value> {
    let topic = text(args, "topic");
    match guide::reference(&topic) {
        Some(text) => Ok(json!({ "topic": topic, "text": text })),
        // A bad topic is the model's to fix, so it gets the list back rather than an apology.
        None => Err(AiError::Tool(format!(
            "no reference topic called {topic}; the topics are {}",
            guide::topics().join(", ")
        ))),
    }
}

/// An absent guide is an answer, not a failure: coverage is deliberately partial, and the prompt
/// turns "no guide" into "I do not know how that works" rather than into a plausible invention.
pub(super) fn app_user_guide(_context: &ToolContext, args: &Value) -> AiResult<Value> {
    let screen = text(args, "screen");
    Ok(json!({ "screen": screen, "text": guide::guide(&screen) }))
}
