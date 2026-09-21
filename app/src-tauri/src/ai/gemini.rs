//! Google's Gemini API: `POST {base}/models/{model}:streamGenerateContent?alt=sse`.
//! Knows nothing about Tauri — see the module-level note in `ai/mod.rs`.
//!
//! Three things here are Gemini's alone. A function call carries **no id**: calls are matched by
//! name, so this adapter mints one (`name#index`) and reads the name back out of it when the
//! result is replayed. A thinking model signs its steps, and a follow-up handing a signed call
//! back **without** its `thoughtSignature` degrades or is refused — which is the whole reason
//! `Block::ToolCall::signature` exists. And thinking is billed apart from the answer
//! (`thoughtsTokenCount` is not inside `candidatesTokenCount`), so the two are added here to
//! keep this app's one rule: reasoning is part of output.

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

impl AiProvider for GeminiProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn> {
        let body = build_request(request);
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
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(AiError::RateLimit {
                message: "rate limited by the provider".into(),
                retry_after,
            });
        }
        if status >= 400 {
            let text = response.body_mut().read_to_string().unwrap_or_default();
            return Err(super::http_error(status, &text));
        }

        read_stream(BufReader::new(response.body_mut().as_reader()), sink, cancelled)
    }
}

/// The stream, separated from the socket so it can be replayed from a recorded fixture.
///
/// Every chunk is a whole `GenerateContentResponse` holding whatever parts are ready: text in
/// fragments, a function call in one piece. `usageMetadata` is a running total, so the last one
/// wins rather than being added up.
fn read_stream<R: std::io::BufRead>(
    reader: R,
    sink: &mut dyn FnMut(AiEvent),
    cancelled: &dyn Fn() -> bool,
) -> AiResult<AiTurn> {
    let mut text = String::new();
    let mut thinking = String::new();
    let mut thinking_signature: Option<String> = None;
    let mut calls: Vec<Block> = Vec::new();
    let mut searched: Vec<String> = Vec::new();
    let mut usage = Usage::default();
    let mut finish = String::new();

    for event in SseReader::new(reader) {
        if cancelled() {
            return Ok(turn(
                text,
                thinking,
                thinking_signature,
                calls,
                searched,
                StopReason::Cancelled,
                Usage::default(),
            ));
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
        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for part in parts {
                let signature = part["thoughtSignature"].as_str().map(str::to_string);
                if let Some(call) = part.get("functionCall").filter(|c| c.is_object()) {
                    let name = call["name"].as_str().unwrap_or_default().to_string();
                    calls.push(Block::ToolCall {
                        // Gemini names no call; the index keeps two calls of one tool apart.
                        id: format!("{name}#{}", calls.len()),
                        name,
                        args_json: call["args"].to_string(),
                        signature,
                    });
                    continue;
                }
                let Some(chunk) = part["text"].as_str().filter(|c| !c.is_empty()) else {
                    // A part carrying nothing but a signature still carries it.
                    if signature.is_some() {
                        thinking_signature = signature;
                    }
                    continue;
                };
                // A thought part is the same shape as an answer part with one flag on it.
                if part["thought"].as_bool().unwrap_or(false) {
                    thinking.push_str(chunk);
                    if signature.is_some() {
                        thinking_signature = signature;
                    }
                    sink(AiEvent::Reasoning {
                        text: chunk.to_string(),
                    });
                } else {
                    text.push_str(chunk);
                    sink(AiEvent::Text {
                        text: chunk.to_string(),
                    });
                }
            }
        }
        // The search is the provider's own: what comes back is the query it ran, not results to
        // hand anywhere.
        if let Some(queries) = candidate["groundingMetadata"]["webSearchQueries"].as_array() {
            for query in queries.iter().filter_map(|q| q.as_str()) {
                if !searched.iter().any(|seen| seen == query) {
                    searched.push(query.to_string());
                    sink(AiEvent::Searching {
                        query: query.to_string(),
                    });
                }
            }
        }
        // A prompt blocked outright has no candidate at all, only this.
        if data["promptFeedback"]["blockReason"].is_string() {
            finish = "SAFETY".to_string();
        }
        if let Some(reason) = candidate["finishReason"].as_str() {
            finish = reason.to_string();
        }
    }

    let stop = match finish.as_str() {
        "MAX_TOKENS" => StopReason::MaxTokens,
        "SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII" | "RECITATION" | "IMAGE_SAFETY" => {
            StopReason::Refusal
        }
        "MALFORMED_FUNCTION_CALL" | "UNEXPECTED_TOOL_CALL" => StopReason::Error(finish.to_lowercase()),
        _ => {
            if calls.is_empty() {
                StopReason::EndTurn
            } else {
                StopReason::ToolUse
            }
        }
    };
    Ok(turn(
        text,
        thinking,
        thinking_signature,
        calls,
        searched,
        stop,
        usage,
    ))
}

#[allow(clippy::too_many_arguments)]
fn turn(
    text: String,
    thinking: String,
    thinking_signature: Option<String>,
    calls: Vec<Block>,
    searched: Vec<String>,
    stop_reason: StopReason,
    usage: Usage,
) -> AiTurn {
    let mut blocks = Vec::new();
    if !thinking.is_empty() {
        blocks.push(Block::Reasoning {
            text: thinking,
            signature: thinking_signature,
        });
    }
    for query in searched {
        blocks.push(Block::WebSearch { query });
    }
    if !text.is_empty() {
        blocks.push(Block::Text { text });
    }
    blocks.extend(calls);
    AiTurn {
        blocks,
        stop_reason,
        usage,
    }
}

/// Gemini counts thinking *beside* the answer rather than inside it, so the two are added: this
/// app's `Usage` says reasoning is part of output, and one provider must not mean something else
/// by the same field (ADR-0041).
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

fn build_request(request: &AiRequest) -> Value {
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
    if request.web_search {
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

/// Every field of a Gemini schema, and nothing else. This API takes a **subset of OpenAPI 3.0**
/// rather than JSON Schema, and answers anything outside it with a 400 naming the field — so the
/// list is an allowlist: a keyword nobody here has heard of is dropped rather than forwarded.
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

/// The catalogue's JSON Schema as Gemini will accept it.
///
/// Two differences do the damage. `additionalProperties: false` — which OpenAI's strict mode
/// requires — is not a field here at all. And a nullable argument is written `type: [T, "null"]`
/// in JSON Schema but `type: T, nullable: true` here, where a list where a string belongs is a
/// parse error rather than a warning. Everything else is copied through untouched.
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

/// One neutral turn becomes one `contents` entry. A tool result is a `user` turn here — Gemini
/// has no third role — and `functionResponse` names the function rather than a call id, which is
/// why the id this adapter minted carries the name in it.
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
                // The result is an object here, not a string and not a list: what a tool
                // answered is JSON, and anything that is not an object — a bare array, a number,
                // a refusal in prose — gets one field of its own rather than being refused.
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
mod tests {
    use super::*;

    fn request() -> AiRequest {
        AiRequest {
            system: "be terse".into(),
            context: "Today is 2026-09-18.".into(),
            model: "gemini-3.1-pro".into(),
            effort: Effort::High,
            messages: vec![(Role::User, vec![Block::Text { text: "hi".into() }])],
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
        }
    }

    #[test]
    fn the_system_text_is_its_own_field_and_the_moving_part_goes_last() {
        let body = build_request(&request());
        assert_eq!(
            body["systemInstruction"]["parts"][0]["text"],
            "be terse\n\nToday is 2026-09-18."
        );
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "high"
        );
        // The summary is asked for only when it was asked for: it costs output tokens.
        assert!(
            body["generationConfig"]["thinkingConfig"]
                .get("includeThoughts")
                .is_none()
        );
        assert!(body.get("tools").is_none());
    }

    /// The catalogue is written for OpenAI's strict mode, which this API shares no vocabulary
    /// with: both differences below come back as a 400 naming the field.
    #[test]
    fn a_strict_json_schema_becomes_one_this_api_accepts() {
        let translated = schema(&json!({
            "type": "object",
            "additionalProperties": false,
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "required": ["symbol"],
            "properties": {
                "symbol": { "type": ["string", "null"], "description": "A ticker." },
                "direction": { "type": ["string", "null"], "enum": ["UP", "DOWN", null] },
                "when": { "type": "string", "format": "uri" },
                "rows": {
                    "type": "array",
                    "items": { "type": ["integer", "null"] },
                },
            },
        }));

        assert!(translated.get("additionalProperties").is_none());
        assert!(translated.get("$schema").is_none());
        assert_eq!(translated["required"][0], "symbol");
        // A union becomes a type and a flag, because a list where a string belongs is a parse
        // error rather than a warning.
        assert_eq!(translated["properties"]["symbol"]["type"], "string");
        assert_eq!(translated["properties"]["symbol"]["nullable"], true);
        assert_eq!(
            translated["properties"]["direction"]["enum"],
            json!(["UP", "DOWN"])
        );
        // An unknown format is dropped: the schema is no less true for saying `string`.
        assert!(translated["properties"]["when"].get("format").is_none());
        assert_eq!(translated["properties"]["rows"]["items"]["type"], "integer");
        assert_eq!(translated["properties"]["rows"]["items"]["nullable"], true);
    }

    #[test]
    fn a_tool_that_takes_nothing_declares_no_parameters() {
        let request = AiRequest {
            tools: vec![super::super::ToolDef {
                name: "portfolio_overview".into(),
                description: "What the portfolio is worth".into(),
                schema: json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            }],
            ..request()
        };
        let body = build_request(&request);
        let declaration = &body["tools"][0]["functionDeclarations"][0];
        assert_eq!(declaration["name"], "portfolio_overview");
        assert!(declaration.get("parameters").is_none());
    }

    #[test]
    fn a_tool_result_goes_back_named_rather_than_by_id() {
        let content = content_of(
            Role::Model,
            &[Block::ToolResult {
                call_id: "portfolio_overview#1".into(),
                content: "{\"value\":\"1\"}".into(),
            }],
        )
        .unwrap();
        // Gemini has no tool role: a result is said by the user side of the conversation.
        assert_eq!(content["role"], "user");
        assert_eq!(
            content["parts"][0]["functionResponse"]["name"],
            "portfolio_overview"
        );
        assert_eq!(content["parts"][0]["functionResponse"]["response"]["value"], "1");
    }

    #[test]
    fn an_answer_that_is_not_an_object_is_given_a_field_of_its_own() {
        for content in ["[1,2]", "not json at all"] {
            let value = content_of(
                Role::Model,
                &[Block::ToolResult {
                    call_id: "positions_list#0".into(),
                    content: content.into(),
                }],
            )
            .unwrap();
            let response = &value["parts"][0]["functionResponse"]["response"];
            assert!(response.is_object(), "{content} was sent as {response}");
        }
    }

    #[test]
    fn a_signed_call_is_handed_back_with_its_signature() {
        let content = content_of(
            Role::Model,
            &[Block::ToolCall {
                id: "positions_list#0".into(),
                name: "positions_list".into(),
                args_json: "{\"period\":\"ONE_YEAR\"}".into(),
                signature: Some("sig-1".into()),
            }],
        )
        .unwrap();
        assert_eq!(content["role"], "model");
        assert_eq!(content["parts"][0]["thoughtSignature"], "sig-1");
        assert_eq!(content["parts"][0]["functionCall"]["args"]["period"], "ONE_YEAR");
    }

    #[test]
    fn text_thoughts_and_a_call_come_out_of_one_stream() {
        let raw = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"weighing\",\"thought\":true}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"he\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"llo\"}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"functionCall\":{\"name\":\"a\",\"args\":{\"x\":1}},\"thoughtSignature\":\"sig\"}]},\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":4,\"cachedContentTokenCount\":2,\"thoughtsTokenCount\":6}}\n\n",
        );
        let mut streamed = String::new();
        let mut counted = Vec::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| match event {
                AiEvent::Text { text } => streamed.push_str(&text),
                AiEvent::Usage { usage } => counted.push(usage),
                _ => {}
            },
            &|| false,
        )
        .unwrap();

        assert_eq!(streamed, "hello");
        assert_eq!(turn.stop_reason, StopReason::ToolUse);
        assert_eq!(
            turn.blocks,
            vec![
                Block::Reasoning {
                    text: "weighing".into(),
                    signature: None,
                },
                Block::Text { text: "hello".into() },
                Block::ToolCall {
                    id: "a#0".into(),
                    name: "a".into(),
                    args_json: "{\"x\":1}".into(),
                    signature: Some("sig".into()),
                },
            ]
        );
        // Thinking is counted beside the answer on the wire and inside it here.
        assert_eq!(counted, vec![turn.usage]);
        assert_eq!(turn.usage.output_tokens, 10);
        assert_eq!(turn.usage.reasoning_tokens, 6);
        assert_eq!(turn.usage.cached_tokens, 2);
    }

    #[test]
    fn a_search_the_provider_ran_is_recorded_once() {
        let raw = concat!(
            "data: {\"candidates\":[{\"groundingMetadata\":{\"webSearchQueries\":[\"gold price\"]}}]}\n\n",
            "data: {\"candidates\":[{\"groundingMetadata\":{\"webSearchQueries\":[\"gold price\"]},\"content\":{\"parts\":[{\"text\":\"up\"}]},\"finishReason\":\"STOP\"}]}\n\n",
        );
        let mut searching = 0;
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| {
                if matches!(event, AiEvent::Searching { .. }) {
                    searching += 1;
                }
            },
            &|| false,
        )
        .unwrap();
        assert_eq!(
            searching, 1,
            "the same query repeated in every chunk is one search"
        );
        assert_eq!(
            turn.blocks,
            vec![
                Block::WebSearch {
                    query: "gold price".into()
                },
                Block::Text { text: "up".into() },
            ]
        );
    }
}
