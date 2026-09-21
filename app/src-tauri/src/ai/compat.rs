//! The OpenAI **chat completions** wire: `POST {base}/chat/completions`, bearer key, SSE deltas.
//! Knows nothing about Tauri — see the module-level note in `ai/mod.rs`.
//!
//! This is what "OpenAI-compatible" means everywhere outside OpenAI itself. OpenRouter, Groq,
//! Together, DeepSeek, Mistral, xAI, vLLM, llama.cpp, LM Studio, Ollama and every LiteLLM proxy
//! answer this one shape, which is why the custom provider speaks it by default.
//!
//! Unlike the Responses API, nothing here hands over a finished turn: the answer exists only as
//! deltas, so the blocks are assembled as they arrive — text into one buffer, each tool call
//! into the slot its `index` names, because arguments arrive one fragment at a time and two
//! calls interleave. A server that reports usage does so on the last chunk, asked for with
//! `stream_options`; one that ignores that field simply reports zeros, which is what "not
//! counted" looks like everywhere else in this app.

use super::sse::SseReader;
use super::{
    AiError, AiEvent, AiProvider, AiRequest, AiResult, AiTurn, Block, Effort, Role, StopReason, Usage,
};
use serde_json::{Value, json};
use std::io::BufReader;

pub struct CompatProvider {
    key: String,
    base: String,
}

impl CompatProvider {
    pub fn new(key: impl Into<String>, base: impl Into<String>) -> Self {
        CompatProvider {
            key: key.into(),
            base: base.into(),
        }
    }
}

impl AiProvider for CompatProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn> {
        let body = build_request(request);
        let bytes = serde_json::to_vec(&body).map_err(|e| AiError::Storage(e.to_string()))?;

        // Errors as data, not `Err`: a 401/429 body carries headers and text we want to read.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();

        let mut response = agent
            .post(format!("{}/chat/completions", self.base))
            .header("Authorization", &format!("Bearer {}", self.key))
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

/// A tool call being assembled. The name arrives once, the arguments in fragments, and the whole
/// thing is identified by its position in the chunk rather than by its id — which some servers
/// only send with the first fragment.
#[derive(Default)]
struct PartialCall {
    id: String,
    name: String,
    args: String,
}

/// The stream, separated from the socket so it can be replayed from a recorded fixture.
fn read_stream<R: std::io::BufRead>(
    reader: R,
    sink: &mut dyn FnMut(AiEvent),
    cancelled: &dyn Fn() -> bool,
) -> AiResult<AiTurn> {
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut calls: Vec<PartialCall> = Vec::new();
    let mut finish = String::new();
    let mut usage = Usage::default();

    for event in SseReader::new(reader) {
        if cancelled() {
            return Ok(turn(
                text,
                reasoning,
                calls,
                StopReason::Cancelled,
                Usage::default(),
            ));
        }
        let event = event.map_err(|e| AiError::Network(e.to_string()))?;
        // The chat wire names no events; `[DONE]` is how it says the answer is over.
        if event.data.trim() == "[DONE]" {
            break;
        }
        let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
            continue; // a keep-alive comment or a field this adapter does not model
        };
        // A gateway may answer 200 and put the failure in the stream.
        if let Some(message) = data["error"]["message"].as_str() {
            return Err(AiError::Provider(message.to_string()));
        }

        if let Some(counted) = usage_of(&data["usage"]) {
            usage = counted;
            sink(AiEvent::Usage { usage });
        }

        let delta = &data["choices"][0]["delta"];
        if let Some(chunk) = delta["content"].as_str().filter(|c| !c.is_empty()) {
            text.push_str(chunk);
            sink(AiEvent::Text {
                text: chunk.to_string(),
            });
        }
        // `reasoning_content` is DeepSeek's spelling and the one vLLM and most gateways copied;
        // `reasoning` is OpenRouter's. Neither is in OpenAI's own schema, so both are optional.
        if let Some(chunk) = delta["reasoning_content"]
            .as_str()
            .or_else(|| delta["reasoning"].as_str())
            .filter(|c| !c.is_empty())
        {
            reasoning.push_str(chunk);
            sink(AiEvent::Reasoning {
                text: chunk.to_string(),
            });
        }
        if let Some(fragments) = delta["tool_calls"].as_array() {
            for fragment in fragments {
                let at = fragment["index"].as_u64().unwrap_or(0) as usize;
                while calls.len() <= at {
                    calls.push(PartialCall::default());
                }
                let call = &mut calls[at];
                if let Some(id) = fragment["id"].as_str() {
                    call.id = id.to_string();
                }
                if let Some(name) = fragment["function"]["name"].as_str() {
                    call.name.push_str(name);
                }
                if let Some(args) = fragment["function"]["arguments"].as_str() {
                    call.args.push_str(args);
                }
            }
        }
        if let Some(reason) = data["choices"][0]["finish_reason"].as_str() {
            finish = reason.to_string();
        }
    }

    let stop = match finish.as_str() {
        "length" => StopReason::MaxTokens,
        "content_filter" => StopReason::Refusal,
        // Everything else — `stop`, `tool_calls`, or a server that reports nothing — is decided
        // by what actually arrived: a turn holding a call is a turn waiting on a tool.
        _ => {
            if calls.is_empty() {
                StopReason::EndTurn
            } else {
                StopReason::ToolUse
            }
        }
    };
    Ok(turn(text, reasoning, calls, stop, usage))
}

/// The blocks in the order a turn holds them: what it thought, what it said, what it asked for.
fn turn(
    text: String,
    reasoning: String,
    calls: Vec<PartialCall>,
    stop_reason: StopReason,
    usage: Usage,
) -> AiTurn {
    let mut blocks = Vec::new();
    if !reasoning.is_empty() {
        blocks.push(Block::Reasoning {
            text: reasoning,
            signature: None,
        });
    }
    if !text.is_empty() {
        blocks.push(Block::Text { text });
    }
    for call in calls {
        if call.name.is_empty() {
            continue; // a slot the server opened and never filled
        }
        blocks.push(Block::ToolCall {
            id: call.id,
            name: call.name,
            args_json: if call.args.is_empty() {
                "{}".to_string()
            } else {
                call.args
            },
            signature: None,
        });
    }
    AiTurn {
        blocks,
        stop_reason,
        usage,
    }
}

/// `usage` is absent from every chunk but the last, and absent altogether from a server that
/// ignores `stream_options`. Both cached and reasoning counts are extensions rather than part of
/// the original schema, so a server reporting neither leaves them at zero.
fn usage_of(usage: &Value) -> Option<Usage> {
    if !usage.is_object() {
        return None;
    }
    Some(Usage {
        input_tokens: usage["prompt_tokens"].as_i64().unwrap_or(0),
        output_tokens: usage["completion_tokens"].as_i64().unwrap_or(0),
        cached_tokens: usage["prompt_tokens_details"]["cached_tokens"]
            .as_i64()
            .unwrap_or(0),
        reasoning_tokens: usage["completion_tokens_details"]["reasoning_tokens"]
            .as_i64()
            .unwrap_or(0),
    })
}

fn build_request(request: &AiRequest) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": instructions(request) })];
    for (role, blocks) in &request.messages {
        messages.extend(turn_to_messages(*role, blocks));
    }

    let tools: Vec<Value> = request
        .tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.schema,
                },
            })
        })
        .collect();

    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
        // Asked for explicitly: without it the chat wire reports nothing at all, and a turn
        // whose cost is unknown is a turn the panel has to lie about.
        "stream_options": { "include_usage": true },
    });
    // Omitted rather than sent empty: a server given `tools: []` may answer that the list is
    // invalid, and a chat with no tools is a plain conversation.
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    // Only a thinking model reads this, and only OpenAI's spelling is standard enough to send.
    // A server that does not know the field ignores it, which is the behaviour we want from it.
    body["reasoning_effort"] = json!(effort_str(request.effort));
    // Sent only when asked for: a local server's own ceiling is usually the model's context, and
    // `max_tokens` is the spelling every compatible server reads.
    if let Some(limit) = request.max_output {
        body["max_tokens"] = json!(limit);
    }
    body
}

/// The moving part goes last, for the reason it does everywhere: the prefix cache matches from
/// the start, so a date above the fixed text would cost the whole prompt on every request.
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

/// One neutral turn becomes one or more wire messages: a tool result is a message of its own
/// with its own role, and the calls that asked for it hang off the assistant message.
fn turn_to_messages(role: Role, blocks: &[Block]) -> Vec<Value> {
    let mut messages = Vec::new();
    let mut text = String::new();
    let mut calls: Vec<Value> = Vec::new();

    for block in blocks {
        match block {
            Block::Text { text: chunk } => text.push_str(chunk),
            Block::ToolCall {
                id, name, args_json, ..
            } => calls.push(json!({
                "id": id,
                "type": "function",
                "function": { "name": name, "arguments": args_json },
            })),
            Block::ToolResult { call_id, content } => messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": content,
            })),
            // Ours to display, never to replay — the same rule as every other adapter.
            Block::WebSearch { .. } | Block::Reasoning { .. } => {}
        }
    }

    if !text.is_empty() || !calls.is_empty() {
        let wire_role = match role {
            Role::User => "user",
            Role::Model => "assistant",
        };
        let mut message = json!({ "role": wire_role, "content": text });
        if !calls.is_empty() {
            message["tool_calls"] = Value::Array(calls);
        }
        // A tool result that arrived in the same turn as the call is still its own message, and
        // it must follow the call rather than precede it.
        messages.insert(0, message);
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AiRequest {
        AiRequest {
            system: "be terse".into(),
            context: "Today is 2026-09-18.".into(),
            model: "qwen3".into(),
            effort: Effort::Medium,
            messages: vec![(Role::User, vec![Block::Text { text: "hi".into() }])],
            tools: Vec::new(),
            web_search: false,
            reasoning_summary: false,
            max_output: None,
        }
    }

    #[test]
    fn the_system_text_is_one_message_and_the_moving_part_goes_last() {
        let body = build_request(&request());
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "be terse\n\nToday is 2026-09-18.");
        assert_eq!(body["messages"][1]["role"], "user");
        // A server that counts nothing reports nothing unless this is asked for.
        assert_eq!(body["stream_options"]["include_usage"], true);
    }

    #[test]
    fn a_chat_without_tools_sends_no_tool_list_at_all() {
        let body = build_request(&request());
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn a_tool_result_is_its_own_message_after_the_call_that_asked() {
        let messages = turn_to_messages(
            Role::Model,
            &[
                Block::ToolCall {
                    id: "call_1".into(),
                    name: "portfolio_overview".into(),
                    args_json: "{}".into(),
                    signature: None,
                },
                Block::ToolResult {
                    call_id: "call_1".into(),
                    content: "{\"value\":\"1\"}".into(),
                },
            ],
        );
        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(
            messages[0]["tool_calls"][0]["function"]["name"],
            "portfolio_overview"
        );
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "call_1");
    }

    /// Arguments arrive one fragment at a time and two calls interleave, so the assembled turn
    /// is the only place the whole call exists.
    #[test]
    fn interleaved_fragments_assemble_into_whole_tool_calls() {
        let raw = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c1\",\"function\":{\"name\":\"a\",\"arguments\":\"{\\\"x\\\"\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":1,\"id\":\"c2\",\"function\":{\"name\":\"b\",\"arguments\":\"{}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\":1}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":3}}\n\n",
            "data: [DONE]\n\n",
        );
        let mut counted = Vec::new();
        let turn = read_stream(
            std::io::Cursor::new(raw),
            &mut |event| {
                if let AiEvent::Usage { usage } = event {
                    counted.push(usage);
                }
            },
            &|| false,
        )
        .unwrap();

        assert_eq!(turn.stop_reason, StopReason::ToolUse);
        assert_eq!(
            turn.blocks,
            vec![
                Block::ToolCall {
                    id: "c1".into(),
                    name: "a".into(),
                    args_json: "{\"x\":1}".into(),
                    signature: None,
                },
                Block::ToolCall {
                    id: "c2".into(),
                    name: "b".into(),
                    args_json: "{}".into(),
                    signature: None,
                },
            ]
        );
        assert_eq!(counted, vec![turn.usage]);
        assert_eq!(turn.usage.input_tokens, 11);
    }

    #[test]
    fn text_streams_and_the_stream_ends_on_done() {
        let raw = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"he\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"llo\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
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

        assert_eq!(streamed, "hello");
        assert_eq!(turn.stop_reason, StopReason::EndTurn);
        assert_eq!(
            turn.blocks,
            vec![
                Block::Reasoning {
                    text: "thinking".into(),
                    signature: None,
                },
                Block::Text { text: "hello".into() },
            ]
        );
    }

    #[test]
    fn a_failure_inside_a_200_stream_is_still_a_failure() {
        let raw = "data: {\"error\":{\"message\":\"no such model\"}}\n\n";
        let result = read_stream(std::io::Cursor::new(raw), &mut |_| {}, &|| false);
        assert!(matches!(result, Err(AiError::Provider(m)) if m.contains("no such model")));
    }
}
