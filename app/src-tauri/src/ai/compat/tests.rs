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
