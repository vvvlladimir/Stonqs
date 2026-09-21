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
mod tests {
    use super::*;

    #[test]
    fn a_text_turn_becomes_one_message_item_per_role() {
        let request = AiRequest {
            system: "be terse".into(),
            context: "Today is 2026-09-16.".into(),
            model: "gpt-5.1".into(),
            effort: Effort::Medium,
            messages: vec![
                (Role::User, vec![Block::Text { text: "hi".into() }]),
                (Role::Model, vec![Block::Text { text: "hello".into() }]),
            ],
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
        };
        let body = build_request(&request);
        // The moving part is rendered after the fixed one, so the cached prefix survives it.
        assert_eq!(body["instructions"], "be terse\n\nToday is 2026-09-16.");
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(body["input"][1]["role"], "assistant");
        assert_eq!(body["input"][1]["content"][0]["type"], "output_text");
    }

    #[test]
    fn a_tool_call_is_its_own_item_not_nested_in_a_message() {
        let blocks = vec![Block::ToolCall {
            id: "call_1".into(),
            name: "portfolio_overview".into(),
            args_json: "{}".into(),
            signature: None,
        }];
        let items = turn_to_items(Role::Model, &blocks);
        assert_eq!(items[0]["type"], "function_call");
        assert_eq!(items[0]["call_id"], "call_1");
        assert!(items[0].get("role").is_none());
    }

    #[test]
    fn the_final_response_object_is_the_source_of_the_turn_not_the_deltas() {
        let response = json!({
            "status": "completed",
            "output": [
                { "type": "message", "role": "assistant", "content": [{ "type": "output_text", "text": "hello" }] }
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 2,
                "input_tokens_details": { "cached_tokens": 4 },
                "output_tokens_details": { "reasoning_tokens": 1 },
            },
        });
        let turn = turn_from_response(&response).unwrap();
        assert_eq!(turn.blocks, vec![Block::Text { text: "hello".into() }]);
        assert_eq!(turn.stop_reason, StopReason::EndTurn);
        assert_eq!(turn.usage.cached_tokens, 4);
        // Thinking is billed as output and reported apart from it; both figures are kept.
        assert_eq!(turn.usage.output_tokens, 2);
        assert_eq!(turn.usage.reasoning_tokens, 1);
    }

    /// The Responses API counts only on `response.completed`, so this is the one moment the
    /// panel can be told anything at all about what the request cost.
    #[test]
    fn the_finished_response_reports_what_it_cost_through_the_sink() {
        let raw = include_str!("../../tests/fixtures/openai_two_parallel_tool_calls.sse");
        let mut reported = Vec::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| {
                if let AiEvent::Usage { usage } = event {
                    reported.push(usage);
                }
            },
            &|| false,
        )
        .unwrap();

        assert_eq!(reported, vec![turn.usage]);
        assert!(turn.usage.input_tokens > 0, "the fixture carries a usage block");
    }

    /// The stream a real turn with two parallel tool calls produces: their argument deltas are
    /// interleaved and out of order, which is exactly what makes concatenating in arrival order
    /// wrong. Reading the finished items off `response.completed` is what makes it not matter.
    #[test]
    fn a_recorded_stream_with_two_parallel_tool_calls_replays_into_one_turn() {
        let raw = include_str!("../../tests/fixtures/openai_two_parallel_tool_calls.sse");
        let mut streamed = String::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| {
                if let AiEvent::Text { text } = event {
                    streamed.push_str(&text);
                }
            },
            &|| false,
        )
        .unwrap();

        assert_eq!(streamed, "Let me check both.", "deltas arrive in order, whole");
        assert_eq!(
            turn.blocks,
            vec![
                Block::Text {
                    text: "Let me check both.".into()
                },
                Block::ToolCall {
                    id: "call_a".into(),
                    name: "portfolio_overview".into(),
                    args_json: "{}".into(),
                    signature: None,
                },
                Block::ToolCall {
                    id: "call_b".into(),
                    name: "positions_list".into(),
                    args_json: r#"{"limit":5}"#.into(),
                    signature: None,
                },
            ]
        );
        assert_eq!(turn.stop_reason, StopReason::ToolUse);
        assert_eq!(turn.usage.cached_tokens, 1024);
    }

    #[test]
    fn a_stop_keeps_the_text_that_already_arrived_and_is_not_an_error() {
        let raw = include_str!("../../tests/fixtures/openai_two_parallel_tool_calls.sse");
        // Cancelled from the very first poll, so nothing is read at all.
        let turn = read_stream(std::io::Cursor::new(raw), &mut |_| {}, &|| true).unwrap();
        assert_eq!(turn.stop_reason, StopReason::Cancelled);
        assert!(turn.blocks.is_empty());
    }

    #[test]
    fn a_tool_is_declared_in_the_flat_responses_shape_not_the_nested_one() {
        let request = AiRequest {
            system: String::new(),
            context: String::new(),
            model: "gpt-5.1".into(),
            effort: Effort::Low,
            messages: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
            tools: vec![super::super::ToolDef {
                name: "portfolio_overview".into(),
                description: "what it is worth".into(),
                schema: json!({ "type": "object", "properties": {} }),
            }],
        };
        let body = build_request(&request);
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["name"], "portfolio_overview");
        assert_eq!(body["tools"][0]["strict"], true);
        assert!(
            body["tools"][0].get("function").is_none(),
            "that nesting is Chat Completions, not Responses"
        );
    }

    #[test]
    fn a_max_output_tokens_cutoff_is_not_a_refusal() {
        let response = json!({
            "status": "incomplete",
            "output": [],
            "incomplete_details": { "reason": "max_output_tokens" },
            "usage": { "input_tokens": 1, "output_tokens": 1, "input_tokens_details": { "cached_tokens": 0 } },
        });
        let turn = turn_from_response(&response).unwrap();
        assert_eq!(turn.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn a_refusal_part_completes_the_response_but_is_a_refusal_with_its_words_kept() {
        let response = json!({
            "status": "completed",
            "output": [{
                "type": "message",
                "content": [{ "type": "refusal", "refusal": "I can't help with that." }],
            }],
            "usage": { "input_tokens": 1, "output_tokens": 1, "input_tokens_details": { "cached_tokens": 0 } },
        });
        let turn = turn_from_response(&response).unwrap();
        assert_eq!(turn.stop_reason, StopReason::Refusal);
        assert_eq!(
            turn.blocks,
            [Block::Text {
                text: "I can't help with that.".into()
            }]
        );
    }

    #[test]
    fn an_http_error_reads_the_provider_s_own_line_not_the_body() {
        let body =
            r#"{"error":{"message":"The model `gpt-9` does not exist","type":"invalid_request_error"}}"#;
        let AiError::Provider(message) = crate::ai::http_error(404, body) else {
            panic!("a 404 is the provider's answer");
        };
        assert_eq!(message, "http 404: The model `gpt-9` does not exist");
    }

    #[test]
    fn a_reasoning_summary_streams_and_lands_as_one_block_but_is_never_sent_back() {
        let raw = include_str!("../../tests/fixtures/openai_reasoning_summary.sse");
        let mut thought = String::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| {
                if let AiEvent::Reasoning { text } = event {
                    thought.push_str(&text);
                }
            },
            &|| false,
        )
        .unwrap();

        // Shown as it arrives, and the two summary parts are paragraphs of one summary.
        assert_eq!(thought, "Checking the holdings first.Then the return.");
        assert_eq!(
            turn.blocks,
            vec![
                Block::Reasoning {
                    text: "Checking the holdings first.\n\nThen the return.".into(),
                    signature: None,
                },
                Block::Text {
                    text: "You are up 4%.".into()
                },
            ]
        );

        // A summary belongs to the step that produced it: replaying it would hand the model
        // somebody else's notes as if they were its own.
        let request = AiRequest {
            system: String::new(),
            context: String::new(),
            model: "gpt-5.1".into(),
            effort: Effort::Medium,
            messages: vec![(Role::Model, turn.blocks.clone())],
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: true,
            max_output: None,
        };
        let body = build_request(&request);
        assert_eq!(
            body["input"].as_array().unwrap().len(),
            1,
            "only the answer is replayed"
        );
        assert_eq!(body["input"][0]["content"][0]["text"], "You are up 4%.");
    }

    /// Asked for only when the user turned it on: an account that is not cleared for summaries
    /// is refused the whole request, which would break a chat over a feature nobody asked for.
    #[test]
    fn the_summary_is_requested_only_when_it_was_switched_on() {
        let base = AiRequest {
            system: String::new(),
            context: String::new(),
            model: "gpt-5.1".into(),
            effort: Effort::Medium,
            messages: Vec::new(),
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
        };
        assert!(build_request(&base)["reasoning"].get("summary").is_none());

        let asked = AiRequest {
            reasoning_summary: true,
            max_output: None,
            ..base
        };
        assert_eq!(build_request(&asked)["reasoning"]["summary"], "auto");
    }
}
