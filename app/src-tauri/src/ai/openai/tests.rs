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
    let raw = include_str!("../../../tests/fixtures/openai_two_parallel_tool_calls.sse");
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
    let raw = include_str!("../../../tests/fixtures/openai_two_parallel_tool_calls.sse");
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
    let raw = include_str!("../../../tests/fixtures/openai_two_parallel_tool_calls.sse");
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
    let body = r#"{"error":{"message":"The model `gpt-9` does not exist","type":"invalid_request_error"}}"#;
    let AiError::Provider(message) = crate::ai::http_error(404, body) else {
        panic!("a 404 is the provider's answer");
    };
    assert_eq!(message, "http 404: The model `gpt-9` does not exist");
}

#[test]
fn a_reasoning_summary_streams_and_lands_as_one_block_but_is_never_sent_back() {
    let raw = include_str!("../../../tests/fixtures/openai_reasoning_summary.sse");
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
