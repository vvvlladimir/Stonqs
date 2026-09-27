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
    let raw = include_str!("../../../tests/fixtures/anthropic_tool_call.sse");
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
