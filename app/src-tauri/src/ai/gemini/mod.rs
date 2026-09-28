//! Gemini adapter. Function calls carry no id (minted as `name#index`), a signed call must be
//! replayed with its `thoughtSignature`, and thinking is counted beside output, so it is added in.

use super::sse::SseReader;
use super::{
    AiError, AiEvent, AiProvider, AiRequest, AiResult, AiTurn, Block, Effort, Role, StopReason, Usage,
};
use serde_json::{Value, json};
use std::io::BufReader;

pub struct GeminiProvider {
    key: String,
    base: String,
}

impl GeminiProvider {
    pub fn new(key: impl Into<String>) -> Self {
        GeminiProvider::at(key, "https://generativelanguage.googleapis.com/v1beta")
    }

    pub fn at(key: impl Into<String>, base: impl Into<String>) -> Self {
        GeminiProvider {
            key: key.into(),
            base: base.into(),
        }
    }
}

/// One request's outcome before the stream is read: an answer, or Google's 429.
enum Attempt {
    Answered(ureq::http::Response<ureq::Body>),
    Throttled(AiError),
}

/// Models whose search grounding Google refused with a 429 during this run.
static NO_SEARCH: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn search_refused(model: &str) -> bool {
    NO_SEARCH
        .lock()
        .is_ok_and(|models| models.iter().any(|m| m == model))
}

fn refuse_search(model: &str) {
    if let Ok(mut models) = NO_SEARCH.lock()
        && !models.iter().any(|m| m == model)
    {
        models.push(model.to_string());
    }
}

impl GeminiProvider {
    fn post(&self, request: &AiRequest, search: bool) -> AiResult<Attempt> {
        let body = build_request(request, search);
        let bytes = serde_json::to_vec(&body).map_err(|e| AiError::Storage(e.to_string()))?;

        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();

        // `alt=sse` is not optional: without it the same endpoint answers one long JSON array,
        // which cannot be read as it arrives.
        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse",
            self.base, request.model
        );
        let mut response = agent
            .post(url)
            // The header rather than `?key=`: a key on the query string ends up in logs and
            // proxy traces, and this one is the user's.
            .header("x-goog-api-key", &self.key)
            .content_type("application/json")
            .send(&bytes[..])
            .map_err(|e| AiError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status == 401 || status == 403 {
            return Err(AiError::Auth("the provider rejected this key".into()));
        }
        if status == 429 {
            let header = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            let text = response.body_mut().read_to_string().unwrap_or_default();
            return Ok(Attempt::Throttled(throttled(&text, header)));
        }
        if status >= 400 {
            let text = response.body_mut().read_to_string().unwrap_or_default();
            return Err(super::http_error(status, &text));
        }
        Ok(Attempt::Answered(response))
    }
}

impl AiProvider for GeminiProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn> {
        let search = request.web_search && !search_refused(&request.model);
        let mut response = match self.post(request, search)? {
            Attempt::Answered(response) => response,
            // Grounding has its own, often zero, quota: retry once without search and drop it for the run.
            Attempt::Throttled(_) if search => {
                refuse_search(&request.model);
                match self.post(request, false)? {
                    Attempt::Answered(response) => response,
                    Attempt::Throttled(error) => return Err(error),
                }
            }
            Attempt::Throttled(error) => return Err(error),
        };

        read_stream(BufReader::new(response.body_mut().as_reader()), sink, cancelled)
    }
}

/// A daily or never-granted quota is the provider's error, not "try again shortly".
fn throttled(body: &str, header: Option<u64>) -> AiError {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let message = parsed["error"]["message"].as_str().unwrap_or_default();
    let details = parsed["error"]["details"].as_array().cloned().unwrap_or_default();
    let exhausted = message.contains("limit: 0")
        || details.iter().any(|detail| {
            detail["violations"].as_array().is_some_and(|violations| {
                violations
                    .iter()
                    .any(|v| v["quotaId"].as_str().is_some_and(|id| id.contains("PerDay")))
            })
        });
    if exhausted {
        return super::http_error(429, body);
    }
    // `RetryInfo.retryDelay` is `"37s"`; the header, when present, says the same.
    let delay = details
        .iter()
        .find_map(|detail| detail["retryDelay"].as_str())
        .and_then(|delay| delay.trim_end_matches('s').parse::<f64>().ok())
        .map(|seconds| seconds.ceil() as u64);
    AiError::RateLimit {
        message: "rate limited by the provider".into(),
        retry_after: header.or(delay),
    }
}

/// Separated from the socket for fixture replay. `usageMetadata` is a running total: the last wins.
fn read_stream<R: std::io::BufRead>(
    reader: R,
    sink: &mut dyn FnMut(AiEvent),
    cancelled: &dyn Fn() -> bool,
) -> AiResult<AiTurn> {
    let mut turn = Turn::default();
    let mut usage = Usage::default();
    let mut finish = String::new();

    for event in SseReader::new(reader) {
        if cancelled() {
            return Ok(turn.finish(StopReason::Cancelled, Usage::default()));
        }
        let event = event.map_err(|e| AiError::Network(e.to_string()))?;
        let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
            continue;
        };
        if let Some(message) = data["error"]["message"].as_str() {
            return Err(AiError::Provider(message.to_string()));
        }

        if let Some(counted) = usage_of(&data["usageMetadata"]) {
            usage = counted;
            sink(AiEvent::Usage { usage });
        }

        let candidate = &data["candidates"][0];
        for part in candidate["content"]["parts"].as_array().into_iter().flatten() {
            turn.part(part, sink);
        }
        turn.searches(&candidate["groundingMetadata"], sink);
        // A prompt blocked outright has no candidate at all, only this.
        if data["promptFeedback"]["blockReason"].is_string() {
            finish = "SAFETY".to_string();
        }
        if let Some(reason) = candidate["finishReason"].as_str() {
            finish = reason.to_string();
        }
    }

    let stop = stop_reason(&finish, !turn.calls.is_empty());
    Ok(turn.finish(stop, usage))
}

fn stop_reason(finish: &str, called: bool) -> StopReason {
    match finish {
        "MAX_TOKENS" => StopReason::MaxTokens,
        "SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII" | "RECITATION" | "IMAGE_SAFETY" => {
            StopReason::Refusal
        }
        "MALFORMED_FUNCTION_CALL" | "UNEXPECTED_TOOL_CALL" => StopReason::Error(finish.to_lowercase()),
        _ if called => StopReason::ToolUse,
        _ => StopReason::EndTurn,
    }
}

/// What a turn has streamed so far, in the order it becomes blocks.
#[derive(Default)]
struct Turn {
    text: String,
    thinking: String,
    thinking_signature: Option<String>,
    calls: Vec<Block>,
    searched: Vec<String>,
}

impl Turn {
    fn part(&mut self, part: &Value, sink: &mut dyn FnMut(AiEvent)) {
        let signature = part["thoughtSignature"].as_str().map(str::to_string);
        if let Some(call) = part.get("functionCall").filter(|c| c.is_object()) {
            let name = call["name"].as_str().unwrap_or_default().to_string();
            self.calls.push(Block::ToolCall {
                // Gemini names no call; the index keeps two calls of one tool apart.
                id: format!("{name}#{}", self.calls.len()),
                name,
                args_json: call["args"].to_string(),
                signature,
            });
            return;
        }
        let Some(chunk) = part["text"].as_str().filter(|c| !c.is_empty()) else {
            // A part carrying nothing but a signature still carries it.
            if signature.is_some() {
                self.thinking_signature = signature;
            }
            return;
        };
        // A thought part is the same shape as an answer part with one flag on it.
        if part["thought"].as_bool().unwrap_or(false) {
            self.thinking.push_str(chunk);
            if signature.is_some() {
                self.thinking_signature = signature;
            }
            sink(AiEvent::Reasoning {
                text: chunk.to_string(),
            });
        } else {
            self.text.push_str(chunk);
            sink(AiEvent::Text {
                text: chunk.to_string(),
            });
        }
    }

    /// The search is the provider's own: what comes back is the query it ran, not results to
    /// hand anywhere.
    fn searches(&mut self, grounding: &Value, sink: &mut dyn FnMut(AiEvent)) {
        let Some(queries) = grounding["webSearchQueries"].as_array() else {
            return;
        };
        for query in queries.iter().filter_map(|q| q.as_str()) {
            if !self.searched.iter().any(|seen| seen == query) {
                self.searched.push(query.to_string());
                sink(AiEvent::Searching {
                    query: query.to_string(),
                });
            }
        }
    }

    fn finish(self, stop_reason: StopReason, usage: Usage) -> AiTurn {
        let mut blocks = Vec::new();
        if !self.thinking.is_empty() {
            blocks.push(Block::Reasoning {
                text: self.thinking,
                signature: self.thinking_signature,
            });
        }
        for query in self.searched {
            blocks.push(Block::WebSearch { query });
        }
        if !self.text.is_empty() {
            blocks.push(Block::Text { text: self.text });
        }
        blocks.extend(self.calls);
        AiTurn {
            blocks,
            stop_reason,
            usage,
        }
    }
}

/// Thinking is counted beside the answer, so it is added to output (ADR-0041).
fn usage_of(usage: &Value) -> Option<Usage> {
    if !usage.is_object() {
        return None;
    }
    let thoughts = usage["thoughtsTokenCount"].as_i64().unwrap_or(0);
    Some(Usage {
        input_tokens: usage["promptTokenCount"].as_i64().unwrap_or(0),
        output_tokens: usage["candidatesTokenCount"].as_i64().unwrap_or(0) + thoughts,
        cached_tokens: usage["cachedContentTokenCount"].as_i64().unwrap_or(0),
        reasoning_tokens: thoughts,
    })
}

fn build_request(request: &AiRequest, search: bool) -> Value {
    let contents: Vec<Value> = request
        .messages
        .iter()
        .filter_map(|(role, blocks)| content_of(*role, blocks))
        .collect();

    let mut tools: Vec<Value> = Vec::new();
    if !request.tools.is_empty() {
        let declarations: Vec<Value> = request
            .tools
            .iter()
            .map(|tool| {
                let mut declaration = json!({
                    "name": tool.name,
                    "description": tool.description,
                });
                // A tool that takes nothing declares no parameters at all: an empty object
                // schema is a field this API has no use for.
                let parameters = schema(&tool.schema);
                if parameters["properties"]
                    .as_object()
                    .is_some_and(|p| !p.is_empty())
                {
                    declaration["parameters"] = parameters;
                }
                declaration
            })
            .collect();
        tools.push(json!({ "functionDeclarations": declarations }));
    }
    // Google runs this one itself and puts what it found in the model's own context; the queries
    // it ran come back as grounding metadata.
    if search {
        tools.push(json!({ "googleSearch": {} }));
    }

    let mut thinking = json!({ "thinkingLevel": effort_str(request.effort) });
    if request.reasoning_summary {
        thinking["includeThoughts"] = json!(true);
    }

    let mut body = json!({
        "contents": contents,
        "systemInstruction": { "parts": [{ "text": instructions(request) }] },
        "generationConfig": {
            "thinkingConfig": thinking,
            "maxOutputTokens": request.max_output.unwrap_or(super::DEFAULT_MAX_OUTPUT),
        },
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    body
}

/// Gemini takes a subset of OpenAPI 3.0 and rejects unknown fields, so this is an allowlist.
const SCHEMA_FIELDS: &[&str] = &[
    "type",
    "format",
    "title",
    "description",
    "nullable",
    "enum",
    "items",
    "properties",
    "required",
    "minimum",
    "maximum",
    "minItems",
    "maxItems",
    "minLength",
    "maxLength",
    "pattern",
    "default",
    "anyOf",
];

/// The formats this API knows. An unknown one is refused rather than ignored, and a schema is
/// not less true for saying `string` where it said `string, uri`.
const SCHEMA_FORMATS: &[&str] = &["date-time", "enum", "int32", "int64", "float", "double"];

/// JSON Schema → Gemini: no `additionalProperties`, and `[T, "null"]` becomes `T` + `nullable`.
fn schema(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(schema).collect()),
        Value::Object(fields) => {
            let mut out = serde_json::Map::new();
            for (key, field) in fields {
                if !SCHEMA_FIELDS.contains(&key.as_str()) {
                    continue;
                }
                match key.as_str() {
                    "type" => match field {
                        Value::Array(types) => {
                            if types.iter().any(Value::is_null)
                                || types.iter().any(|t| t.as_str() == Some("null"))
                            {
                                out.insert("nullable".into(), json!(true));
                            }
                            let named = types
                                .iter()
                                .filter_map(Value::as_str)
                                .find(|t| *t != "null")
                                .unwrap_or("string");
                            out.insert("type".into(), json!(named));
                        }
                        other => {
                            out.insert("type".into(), other.clone());
                        }
                    },
                    // `null` is a member of a strict-mode enum and a syntax error here; what it
                    // meant is said by `nullable`, which the type above already set.
                    "enum" => {
                        let values: Vec<Value> = field
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|v| !v.is_null())
                            .cloned()
                            .collect();
                        out.insert("enum".into(), Value::Array(values));
                    }
                    "format" => {
                        if field.as_str().is_some_and(|f| SCHEMA_FORMATS.contains(&f)) {
                            out.insert("format".into(), field.clone());
                        }
                    }
                    "properties" => {
                        let mut properties = serde_json::Map::new();
                        for (name, property) in field.as_object().into_iter().flatten() {
                            properties.insert(name.clone(), schema(property));
                        }
                        out.insert("properties".into(), Value::Object(properties));
                    }
                    _ => {
                        out.insert(key.clone(), schema(field));
                    }
                }
            }
            // An enum of strings with no type is a type error waiting to happen.
            if out.contains_key("enum") && !out.contains_key("type") {
                out.insert("type".into(), json!("string"));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// The moving part goes last, as everywhere: the prefix cache matches from the start.
fn instructions(request: &AiRequest) -> String {
    match request.context.trim() {
        "" => request.system.clone(),
        context => format!("{}\n\n{context}", request.system),
    }
}

/// `thinkingLevel` is the Gemini 3 control; the older `thinkingBudget` was a token count, which
/// is a different question than "how hard should this chat think".
fn effort_str(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
    }
}

/// A tool result is a `user` turn; `functionResponse` names the function, hence the minted id.
fn content_of(role: Role, blocks: &[Block]) -> Option<Value> {
    let mut parts: Vec<Value> = Vec::new();
    let mut is_result = false;

    for block in blocks {
        match block {
            Block::Text { text } => parts.push(json!({ "text": text })),
            Block::ToolCall {
                name,
                args_json,
                signature,
                ..
            } => {
                // Both sides of a call are objects here, and anything else is a 400 rather than
                // a coercion: arguments that did not parse into one are sent as none.
                let args = serde_json::from_str::<Value>(args_json)
                    .ok()
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({}));
                let mut part = json!({ "functionCall": { "name": name, "args": args } });
                // Handing a signed step back unsigned is the one thing this provider refuses.
                if let Some(signature) = signature {
                    part["thoughtSignature"] = json!(signature);
                }
                parts.push(part);
            }
            Block::ToolResult { call_id, content } => {
                is_result = true;
                // A result must be an object; anything else is wrapped in one field.
                let answer = serde_json::from_str::<Value>(content)
                    .ok()
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({ "result": content.clone() }));
                parts.push(json!({
                    "functionResponse": { "name": tool_of(call_id), "response": answer },
                }));
            }
            // Ours to display, never to replay — the same rule as every other adapter.
            Block::WebSearch { .. } | Block::Reasoning { .. } => {}
        }
    }

    if parts.is_empty() {
        return None;
    }
    let wire_role = match role {
        Role::Model if !is_result => "model",
        _ => "user",
    };
    Some(json!({ "role": wire_role, "parts": parts }))
}

/// The tool a minted id belongs to: `portfolio_overview#1` is the second call of that tool.
fn tool_of(call_id: &str) -> &str {
    call_id.split('#').next().unwrap_or(call_id)
}

#[cfg(test)]
mod tests;
