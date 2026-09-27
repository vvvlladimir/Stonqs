use super::*;

#[test]
fn a_quota_that_is_gone_for_the_day_is_not_a_throttle() {
    let body = r#"{"error":{"code":429,"message":"You exceeded your current quota. Quota exceeded for metric: generate_content_free_tier_requests, limit: 0, model: gemini-3.8-flash","status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"GenerateRequestsPerDayPerProjectPerModel-FreeTier"}]}]}}"#;
    match throttled(body, None) {
        AiError::Provider(message) => assert!(message.contains("limit: 0"), "{message}"),
        other => panic!("expected the provider's own error, got {other:?}"),
    }
}

#[test]
fn a_per_minute_throttle_says_when_to_come_back() {
    let body = r#"{"error":{"code":429,"message":"Resource exhausted","status":"RESOURCE_EXHAUSTED","details":[{"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"GenerateRequestsPerMinutePerProjectPerModel"}]},{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"36.4s"}]}}"#;
    match throttled(body, None) {
        AiError::RateLimit { retry_after, .. } => assert_eq!(retry_after, Some(37)),
        other => panic!("expected a rate limit, got {other:?}"),
    }
}

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
    let body = build_request(&request(), request().web_search);
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
    let body = build_request(&request, request.web_search);
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
