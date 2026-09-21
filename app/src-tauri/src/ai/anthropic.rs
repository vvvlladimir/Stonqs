//! Anthropic Messages API adapter: JSON request body, SSE event interpretation, error mapping.
//! Knows nothing about Tauri, like the rest of `ai/` outside `keys.rs`.
//!
//! Shaped like `openai.rs` and deliberately different where the wire is: Anthropic has no final
//! envelope carrying the finished message, so the turn is **assembled from the deltas** rather
//! than read off one object at the end. Blocks arrive strictly one at a time (`index` only ever
//! moves forward), which is what makes a single in-progress block enough.

use super::sse::SseReader;
use super::{
    AiError, AiEvent, AiProvider, AiRequest, AiResult, AiTurn, Block, Effort, Role, StopReason, Usage,
};
use serde_json::{Value, json};
use std::io::BufReader;

const VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    key: String,
    /// Everything before `/messages`. Anthropic's own host by default; a gateway that speaks
    /// this wire puts its own address here (`catalog::Wire::Anthropic`).
    base: String,
}

impl AnthropicProvider {
    pub fn new(key: impl Into<String>) -> Self {
        AnthropicProvider::at(key, "https://api.anthropic.com/v1")
    }

    pub fn at(key: impl Into<String>, base: impl Into<String>) -> Self {
        AnthropicProvider {
            key: key.into(),
            base: base.into(),
        }
    }
}

impl AiProvider for AnthropicProvider {
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

        let mut response = agent
            .post(format!("{}/messages", self.base))
            .header("x-api-key", &self.key)
            .header("anthropic-version", VERSION)
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

/// One block while its deltas are still arriving. Anthropic sends blocks in order, so there is
/// never more than one open at a time — the `index` is a check, not a key.
enum Open {
    Text(String),
    Thinking {
        text: String,
        signature: Option<String>,
    },
    ToolUse {
        id: String,
        name: String,
        args: String,
    },
    /// A tool the provider runs itself; only the query is ours to show.
    Search {
        args: String,
    },
    /// A block this adapter shows nothing for (search results, redacted thinking).
    Ignored,
}

fn read_stream<R: std::io::BufRead>(
    reader: R,
    sink: &mut dyn FnMut(AiEvent),
    cancelled: &dyn Fn() -> bool,
) -> AiResult<AiTurn> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut open: Option<Open> = None;
    let mut usage = Usage::default();
    let mut stop_reason = StopReason::EndTurn;
    let mut streamed = String::new();

    for event in SseReader::new(reader) {
        if cancelled() {
            return Ok(cancelled_turn(streamed));
        }
        let event = event.map_err(|e| AiError::Network(e.to_string()))?;
        let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
            continue;
        };

        match event.event.as_str() {
            // What the request cost is split across two events: the input is known when the
            // message starts, the output only as it ends.
            "message_start" => {
                usage = usage_from(&data["message"]["usage"], usage);
                sink(AiEvent::Usage { usage });
            }
            "content_block_start" => {
                open = Some(open_block(&data["content_block"]));
            }
            "content_block_delta" => {
                let delta = &data["delta"];
                match (open.as_mut(), delta["type"].as_str()) {
                    (Some(Open::Text(text)), Some("text_delta")) => {
                        if let Some(chunk) = delta["text"].as_str() {
                            text.push_str(chunk);
                            streamed.push_str(chunk);
                            sink(AiEvent::Text {
                                text: chunk.to_string(),
                            });
                        }
                    }
                    (Some(Open::Thinking { text, .. }), Some("thinking_delta")) => {
                        if let Some(chunk) = delta["thinking"].as_str() {
                            text.push_str(chunk);
                            sink(AiEvent::Reasoning {
                                text: chunk.to_string(),
                            });
                        }
                    }
                    // The signature is the provider's proof that the thinking is its own; it
                    // arrives apart from the text and is kept so the block can be replayed.
                    (Some(Open::Thinking { signature, .. }), Some("signature_delta")) => {
                        if let Some(chunk) = delta["signature"].as_str() {
                            signature.get_or_insert_with(String::new).push_str(chunk);
                        }
                    }
                    (Some(Open::ToolUse { args, .. }), Some("input_json_delta"))
                    | (Some(Open::Search { args }), Some("input_json_delta")) => {
                        if let Some(chunk) = delta["partial_json"].as_str() {
                            args.push_str(chunk);
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                if let Some(block) = open.take().and_then(|o| close_block(o, sink)) {
                    blocks.push(block);
                }
            }
            "message_delta" => {
                usage = usage_from(&data["usage"], usage);
                sink(AiEvent::Usage { usage });
                if let Some(reason) = data["delta"]["stop_reason"].as_str() {
                    stop_reason = stop_from(reason);
                }
            }
            "message_stop" => {
                return Ok(AiTurn {
                    blocks,
                    stop_reason,
                    usage,
                });
            }
            // A failure mid-stream: the body carries the provider's own type, and only two of
            // them mean something different to the user than "the provider said no".
            "error" => {
                let kind = data["error"]["type"].as_str().unwrap_or_default();
                let message = data["error"]["message"]
                    .as_str()
                    .unwrap_or("the provider reported a failure")
                    .to_string();
                return Err(match kind {
                    "authentication_error" | "permission_error" => AiError::Auth(message),
                    "rate_limit_error" => AiError::RateLimit {
                        message,
                        retry_after: None,
                    },
                    _ => AiError::Provider(message),
                });
            }
            _ => {}
        }
    }

    Err(AiError::Network(
        "the connection ended before the response completed".into(),
    ))
}

fn open_block(start: &Value) -> Open {
    match start["type"].as_str() {
        Some("text") => Open::Text(start["text"].as_str().unwrap_or_default().to_string()),
        Some("thinking") => Open::Thinking {
            text: start["thinking"].as_str().unwrap_or_default().to_string(),
            signature: start["signature"].as_str().map(str::to_string),
        },
        Some("tool_use") => Open::ToolUse {
            id: start["id"].as_str().unwrap_or_default().to_string(),
            name: start["name"].as_str().unwrap_or_default().to_string(),
            args: String::new(),
        },
        Some("server_tool_use") => Open::Search { args: String::new() },
        _ => Open::Ignored,
    }
}

fn close_block(open: Open, sink: &mut dyn FnMut(AiEvent)) -> Option<Block> {
    match open {
        Open::Text(text) => (!text.is_empty()).then_some(Block::Text { text }),
        Open::Thinking { text, signature } => {
            (!text.is_empty()).then_some(Block::Reasoning { text, signature })
        }
        Open::ToolUse { id, name, args } => Some(Block::ToolCall {
            id,
            name,
            // An empty argument object arrives as no deltas at all, not as "{}".
            args_json: if args.is_empty() { "{}".into() } else { args },
            // Anthropic signs its thinking, not its tool calls; only Gemini signs these.
            signature: None,
        }),
        Open::Search { args } => {
            let query = serde_json::from_str::<Value>(&args)
                .ok()
                .and_then(|v| v["query"].as_str().map(str::to_string))
                .unwrap_or_default();
            sink(AiEvent::Searching { query: query.clone() });
            Some(Block::WebSearch { query })
        }
        Open::Ignored => None,
    }
}

/// Fields absent from this event keep what an earlier one reported — the input is counted once,
/// at the start, and the output only at the end.
fn usage_from(reported: &Value, mut usage: Usage) -> Usage {
    let read = reported["cache_read_input_tokens"].as_i64();
    let written = reported["cache_creation_input_tokens"].as_i64();
    if let Some(input) = reported["input_tokens"].as_i64() {
        // The provider counts the cached part apart from the rest; the app counts it inside the
        // input, the way OpenAI reports it (ADR-0041).
        usage.input_tokens = input + read.unwrap_or(0) + written.unwrap_or(0);
    }
    if let Some(read) = read {
        usage.cached_tokens = read;
    }
    if let Some(output) = reported["output_tokens"].as_i64() {
        usage.output_tokens = output;
    }
    usage
}

fn stop_from(reason: &str) -> StopReason {
    match reason {
        "end_turn" | "stop_sequence" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "refusal" => StopReason::Refusal,
        // `pause_turn` is the provider asking to be called again to continue its own tool; the
        // loop only continues on a tool call of ours, so the answer so far is what there is.
        "pause_turn" => StopReason::EndTurn,
        other => StopReason::Error(other.to_string()),
    }
}

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

fn build_request(request: &AiRequest) -> Value {
    let messages: Vec<Value> = request
        .messages
        .iter()
        .filter_map(|(role, blocks)| turn_to_message(*role, blocks))
        .collect();

    let mut tools: Vec<Value> = request
        .tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.schema,
            })
        })
        .collect();
    if request.web_search {
        // The dated name is the tool's version, not a release: the older one is accepted by every
        // model that has search at all, and the newer one is not.
        tools.push(json!({ "type": "web_search_20250305", "name": "web_search" }));
    }

    let mut body = json!({
        "model": request.model,
        "max_tokens": request.max_output.unwrap_or(super::DEFAULT_MAX_OUTPUT),
        "system": system_blocks(request),
        "messages": messages,
        "tools": tools,
        "stream": true,
        "output_config": { "effort": effort_str(request.effort) },
    });

    // Asked for only when the user turned it on — and the field is absent rather than disabled,
    // because on the newest models thinking is on by default and only its *display* is ours.
    if request.reasoning_summary {
        body["thinking"] = json!({ "type": "adaptive", "display": "summarized" });
    }
    body
}

/// Two blocks, and the cache breakpoint on the first: tools and system render before the
/// messages, so marking the end of the fixed text caches everything up to it. The moving part
/// goes after it, exactly as in `openai.rs` — the reason is the same, the mechanism explicit.
fn system_blocks(request: &AiRequest) -> Value {
    let mut blocks = vec![json!({
        "type": "text",
        "text": request.system,
        "cache_control": { "type": "ephemeral" },
    })];
    let context = request.context.trim();
    if !context.is_empty() {
        blocks.push(json!({ "type": "text", "text": context }));
    }
    json!(blocks)
}

fn effort_str(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
    }
}

/// One neutral turn is one message here — unlike OpenAI, a tool call is a block inside the
/// assistant's message rather than an item of its own.
fn turn_to_message(role: Role, blocks: &[Block]) -> Option<Value> {
    let content: Vec<Value> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Text { text } => Some(json!({ "type": "text", "text": text })),
            Block::ToolCall {
                id, name, args_json, ..
            } => Some(json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                // The provider wants the arguments as an object; what it sent us was a string.
                "input": serde_json::from_str::<Value>(args_json).unwrap_or_else(|_| json!({})),
            })),
            Block::ToolResult { call_id, content } => Some(json!({
                "type": "tool_result",
                "tool_use_id": call_id,
                "content": content,
            })),
            // Ours to display, not to replay: the provider ran it and holds the results.
            Block::WebSearch { .. } => None,
            // The one place a summary *is* sent back. The provider signs its own thinking and
            // refuses a turn whose tool call arrived without it, so a signed block is replayed
            // exactly as it came; an unsigned one (another provider's, or an older row) is not.
            Block::Reasoning { text, signature } => signature
                .as_ref()
                .map(|signature| json!({ "type": "thinking", "thinking": text, "signature": signature })),
        })
        .collect();

    // A message with no content at all is refused; a turn that held only a web search is one.
    (!content.is_empty()).then(|| {
        json!({
            "role": match role { Role::User => "user", Role::Model => "assistant" },
            "content": content,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AiRequest {
        AiRequest {
            system: "be terse".into(),
            context: "Today is 2026-09-17.".into(),
            model: "claude-opus-5".into(),
            effort: Effort::Medium,
            messages: Vec::new(),
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
        }
    }

    #[test]
    fn the_moving_context_is_a_block_of_its_own_after_the_cached_one() {
        let body = build_request(&request());
        assert_eq!(body["system"][0]["text"], "be terse");
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(body["system"][1]["text"], "Today is 2026-09-17.");
        assert!(body["system"][1].get("cache_control").is_none());
    }

    #[test]
    fn a_tool_call_is_a_block_of_the_assistants_message_with_its_arguments_parsed() {
        let message = turn_to_message(
            Role::Model,
            &[Block::ToolCall {
                id: "toolu_1".into(),
                name: "portfolio_overview".into(),
                args_json: r#"{"period":"YTD"}"#.into(),
                signature: None,
            }],
        )
        .expect("a message");
        assert_eq!(message["role"], "assistant");
        assert_eq!(message["content"][0]["type"], "tool_use");
        assert_eq!(message["content"][0]["input"]["period"], "YTD");
    }

    #[test]
    fn only_a_signed_summary_is_replayed() {
        let signed = turn_to_message(
            Role::Model,
            &[Block::Reasoning {
                text: "checking".into(),
                signature: Some("sig".into()),
            }],
        )
        .expect("a message");
        assert_eq!(signed["content"][0]["type"], "thinking");
        assert_eq!(signed["content"][0]["signature"], "sig");

        // Another provider's summary carries no signature, and a turn made only of one is not
        // a message at all.
        assert!(
            turn_to_message(
                Role::Model,
                &[Block::Reasoning {
                    text: "checking".into(),
                    signature: None,
                }],
            )
            .is_none()
        );
    }

    #[test]
    fn the_turn_is_assembled_from_the_deltas_because_nothing_repeats_it_at_the_end() {
        let raw = include_str!("../../tests/fixtures/anthropic_tool_call.sse");
        let mut streamed = String::new();
        let mut reported = Vec::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| match event {
                AiEvent::Text { text } => streamed.push_str(&text),
                AiEvent::Usage { usage } => reported.push(usage),
                _ => {}
            },
            &|| false,
        )
        .unwrap();

        assert_eq!(streamed, "Let me check.");
        assert_eq!(
            turn.blocks,
            vec![
                Block::Reasoning {
                    text: "The year so far.".into(),
                    signature: Some("sig-abc".into()),
                },
                Block::Text {
                    text: "Let me check.".into()
                },
                Block::ToolCall {
                    id: "toolu_a".into(),
                    name: "portfolio_overview".into(),
                    args_json: r#"{"period":"YTD"}"#.into(),
                    signature: None,
                },
            ]
        );
        assert_eq!(turn.stop_reason, StopReason::ToolUse);
        // The cached part is counted inside the input, and the output lands with the last event.
        assert_eq!(turn.usage.input_tokens, 1200);
        assert_eq!(turn.usage.cached_tokens, 1024);
        assert_eq!(turn.usage.output_tokens, 57);
        assert_eq!(reported.last(), Some(&turn.usage));
    }

    #[test]
    fn a_provider_error_mid_stream_is_its_own_kind_of_failure() {
        let raw = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"rate_limit_error\",\"message\":\"slow down\"}}\n\n";
        let error = read_stream(std::io::Cursor::new(raw), &mut |_| {}, &|| false).unwrap_err();
        assert!(matches!(error, AiError::RateLimit { .. }), "{error:?}");
    }
}
