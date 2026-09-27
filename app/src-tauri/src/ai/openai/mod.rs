//! OpenAI Responses API adapter: JSON request body, SSE event interpretation, error mapping.
//! Knows nothing about Tauri — see the module-level note in `ai/mod.rs`.

use super::sse::SseReader;
use super::{
    AiError, AiEvent, AiProvider, AiRequest, AiResult, AiTurn, Block, Effort, Role, StopReason, Usage,
};
use serde_json::{Value, json};
use std::io::BufReader;

pub struct OpenAiProvider {
    key: String,
    /// Everything before `/responses`. OpenAI's own host by default; a server implementing this
    /// same wire puts its own address here (`catalog::Wire::OpenAiResponses`).
    base: String,
}

impl OpenAiProvider {
    pub fn new(key: impl Into<String>) -> Self {
        OpenAiProvider::at(key, "https://api.openai.com/v1")
    }

    pub fn at(key: impl Into<String>, base: impl Into<String>) -> Self {
        OpenAiProvider {
            key: key.into(),
            base: base.into(),
        }
    }
}

impl AiProvider for OpenAiProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn> {
        let body = build_request(request);
        let bytes = serde_json::to_vec(&body).map_err(|e| AiError::Storage(e.to_string()))?;

        // Errors as data, not `Err`: a 401/429 body carries headers and text we want to read,
        // and the default `Err(StatusCode(_))` throws both away.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();

        let mut response = agent
            .post(format!("{}/responses", self.base))
            .header("Authorization", &format!("Bearer {}", self.key))
            .content_type("application/json")
            .send(&bytes[..])
            .map_err(|e| AiError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status == 401 {
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
/// Deltas are forwarded for the live UI and nothing else. The finished turn is read off the
/// `response.completed` envelope, which already carries every output item — so there is no
/// partial JSON to reassemble and no assumption to get wrong about the order parallel tool
/// calls arrive in.
fn read_stream<R: std::io::BufRead>(
    reader: R,
    sink: &mut dyn FnMut(AiEvent),
    cancelled: &dyn Fn() -> bool,
) -> AiResult<AiTurn> {
    // Kept only so a stop still shows what was already paid for.
    let mut streamed = String::new();

    for event in SseReader::new(reader) {
        if cancelled() {
            return Ok(cancelled_turn(streamed));
        }
        let event = event.map_err(|e| AiError::Network(e.to_string()))?;
        let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data) else {
            continue; // a keep-alive line or a field this adapter does not model
        };

        match event.event.as_str() {
            // A refusal streams as its own part; it is still the model's words to the user.
            "response.output_text.delta" | "response.refusal.delta" => {
                if let Some(delta) = data["delta"].as_str() {
                    streamed.push_str(delta);
                    sink(AiEvent::Text {
                        text: delta.to_string(),
                    });
                }
            }
            "response.reasoning_summary_text.delta" => {
                if let Some(delta) = data["delta"].as_str() {
                    sink(AiEvent::Reasoning {
                        text: delta.to_string(),
                    });
                }
            }
            "response.completed" => {
                let turn = turn_from_response(&data["response"])?;
                // The Responses API counts only once, on this event: there is nothing to report
                // while the answer streams, so the panel's figure lands when the step lands.
                sink(AiEvent::Usage { usage: turn.usage });
                return Ok(turn);
            }
            "response.failed" => {
                let message = data["response"]["error"]["message"]
                    .as_str()
                    .unwrap_or("the provider reported a failure")
                    .to_string();
                return Err(AiError::Provider(message));
            }
            _ => {}
        }
    }

    Err(AiError::Network(
        "the connection ended before the response completed".into(),
    ))
}

/// A stop mid-stream is not a failure: the text already on screen is the turn, and saying so
/// lets the loop persist it instead of throwing away what the user watched arrive.
fn cancelled_turn(streamed: String) -> AiTurn {
    AiTurn {
        blocks: if streamed.is_empty() {
            Vec::new()
        } else {
            vec![Block::Text { text: streamed }]
        },
        stop_reason: StopReason::Cancelled,
        usage: Usage::default(),
    }
}

fn build_request(request: &AiRequest) -> serde_json::Value {
    let input: Vec<serde_json::Value> = request
        .messages
        .iter()
        .flat_map(|(role, blocks)| turn_to_items(*role, blocks))
        .collect();

    // The Responses API's flat form — `{type, name, parameters}` — not Chat Completions' nested
    // `{type: "function", function: {...}}`. `strict` makes the model match the schema instead
    // of occasionally sending arguments that do not parse.
    let tools: Vec<serde_json::Value> = request
        .tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.schema,
                "strict": true,
            })
        })
        .collect();

    // The hosted search tool has no schema: OpenAI runs it and puts the results in the model's
    // own context, so it is declared by name only.
    let mut tools = tools;
    if request.web_search {
        tools.push(json!({ "type": "web_search" }));
    }

    // `summary` is only present when it was asked for: an account not cleared for reasoning
    // summaries is refused the whole request, so the field is absent rather than "none".
    let reasoning = match request.reasoning_summary {
        true => json!({ "effort": effort_str(request.effort), "summary": "auto" }),
        false => json!({ "effort": effort_str(request.effort) }),
    };

    json!({
        "model": request.model,
        "instructions": instructions(request),
        "input": input,
        "tools": tools,
        "stream": true,
        "reasoning": reasoning,
        "max_output_tokens": request.max_output.unwrap_or(super::DEFAULT_MAX_OUTPUT),
    })
}

/// The moving part goes last. The prefix cache matches from the start, so a date or a screen name
/// placed above the fixed text would cost the whole prompt on every request.
fn instructions(request: &AiRequest) -> String {
    match request.context.trim() {
        "" => request.system.clone(),
        context => format!("{}\n\n{context}", request.system),
    }
}

fn effort_str(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
    }
}

/// One neutral turn can expand into several top-level `input` items: a tool call is its own
/// item, not nested inside a message, and only message items carry a `role` at all.
fn turn_to_items(role: Role, blocks: &[Block]) -> Vec<serde_json::Value> {
    let (wire_role, text_type) = match role {
        Role::User => ("user", "input_text"),
        Role::Model => ("assistant", "output_text"),
    };
    blocks
        .iter()
        .map(|block| match block {
            Block::Text { text } => json!({
                "type": "message",
                "role": wire_role,
                "content": [{ "type": text_type, "text": text }],
            }),
            Block::ToolCall {
                id, name, args_json, ..
            } => json!({
                "type": "function_call",
                "call_id": id,
                "name": name,
                "arguments": args_json,
            }),
            Block::ToolResult { call_id, content } => json!({
                "type": "function_call_output",
                "call_id": call_id,
                "output": content,
            }),
            // Ours to display, not to replay: the provider ran it, and its own item ids are not
            // ours to hand back. What the search produced is already in the text of that turn.
            Block::WebSearch { .. } => Value::Null,
            // Same rule, different reason: a reasoning item belongs to the step that produced it
            // and is not valid input for the next one.
            Block::Reasoning { .. } => Value::Null,
        })
        .filter(|item| !item.is_null())
        .collect()
}

/// `response.completed`'s `response` object already carries every finished item — the deltas
/// forwarded to `sink` above are for the live UI only, never re-parsed to build this.
fn turn_from_response(response: &serde_json::Value) -> AiResult<AiTurn> {
    let status = response["status"].as_str().unwrap_or("");
    let blocks = blocks_from_output(&response["output"]);
    let usage = Usage {
        input_tokens: response["usage"]["input_tokens"].as_i64().unwrap_or(0),
        output_tokens: response["usage"]["output_tokens"].as_i64().unwrap_or(0),
        cached_tokens: response["usage"]["input_tokens_details"]["cached_tokens"]
            .as_i64()
            .unwrap_or(0),
        reasoning_tokens: response["usage"]["output_tokens_details"]["reasoning_tokens"]
            .as_i64()
            .unwrap_or(0),
    };
    let stop_reason = match status {
        "completed" if refused(&response["output"]) => StopReason::Refusal,
        "completed" => {
            if blocks.iter().any(|b| matches!(b, Block::ToolCall { .. })) {
                StopReason::ToolUse
            } else {
                StopReason::EndTurn
            }
        }
        "incomplete" => match response["incomplete_details"]["reason"].as_str() {
            Some("max_output_tokens") => StopReason::MaxTokens,
            Some("content_filter") => StopReason::Refusal,
            other => StopReason::Error(other.unwrap_or("incomplete").to_string()),
        },
        other => StopReason::Error(other.to_string()),
    };
    Ok(AiTurn {
        blocks,
        stop_reason,
        usage,
    })
}

/// A refusal is not a status: the response completes, and a `refusal` part stands where the
/// `output_text` would have been.
fn refused(output: &serde_json::Value) -> bool {
    output.as_array().into_iter().flatten().any(|item| {
        item["content"]
            .as_array()
            .is_some_and(|parts| parts.iter().any(|part| part["type"] == "refusal"))
    })
}

fn blocks_from_output(output: &serde_json::Value) -> Vec<Block> {
    let Some(items) = output.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match item["type"].as_str() {
            Some("message") => {
                let text: String = item["content"]
                    .as_array()?
                    .iter()
                    .filter_map(|part| part["text"].as_str().or_else(|| part["refusal"].as_str()))
                    .collect();
                if text.is_empty() {
                    None
                } else {
                    Some(Block::Text { text })
                }
            }
            // One item, several summary parts: they are paragraphs of one summary, so they are
            // joined rather than shown as separate thoughts.
            Some("reasoning") => {
                let text: String = item["summary"]
                    .as_array()?
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                (!text.is_empty()).then_some(Block::Reasoning {
                    text,
                    signature: None,
                })
            }
            Some("web_search_call") => Some(Block::WebSearch {
                query: item["action"]["query"]
                    .as_str()
                    .or_else(|| item["query"].as_str())
                    .unwrap_or_default()
                    .to_string(),
            }),
            Some("function_call") => Some(Block::ToolCall {
                id: item["call_id"].as_str()?.to_string(),
                name: item["name"].as_str()?.to_string(),
                args_json: item["arguments"].as_str().unwrap_or("{}").to_string(),
                signature: None,
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
