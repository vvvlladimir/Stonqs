use super::*;

/// A chat carries its own tool mode, spelled the way every other enum crosses: SCREAMING_SNAKE.
#[test]
fn ai_chat_keys_match_the_typescript_types() {
    let chat = sq_core::model::AiChat {
        id: "chat-1".into(),
        title: "How did I do this year?".into(),
        provider: "openai".into(),
        model: "gpt-5.1".into(),
        tool_mode: sq_core::model::AiToolMode::Ask,
        effort: sq_core::model::AiEffort::Medium,
        created_at: "2026-09-15 10:00:00".into(),
        updated_at: "2026-09-15 10:01:00".into(),
    };
    let json: Value = serde_json::to_value(&chat).unwrap();
    assert_eq!(
        keys(&json),
        [
            "created_at",
            "effort",
            "id",
            "model",
            "provider",
            "title",
            "tool_mode",
            "updated_at"
        ]
    );
    assert_eq!(json["tool_mode"], "ASK");
    assert_eq!(json["effort"], "MEDIUM");
    assert_eq!(
        serde_json::to_value(sq_core::model::AiToolMode::Auto).unwrap(),
        "AUTO"
    );
}

/// Token counts cross as numbers, not strings: they are counts, not money, and nothing in the
/// app does arithmetic on them beyond adding them up.
#[test]
fn ai_usage_keys_match_the_typescript_types() {
    let total = sq_core::model::AiUsageTotal {
        provider: "openai".into(),
        model: "gpt-5.1".into(),
        requests: 3,
        usage: sq_core::model::AiUsage {
            input_tokens: 12_000,
            cached_tokens: 9_216,
            output_tokens: 800,
            reasoning_tokens: 512,
        },
        first_at: "2026-09-15 10:00:00".into(),
        last_at: "2026-09-16 09:30:00".into(),
    };
    let json: Value = serde_json::to_value(&total).unwrap();
    assert_eq!(
        keys(&json),
        ["first_at", "last_at", "model", "provider", "requests", "usage"]
    );
    assert_eq!(
        keys(&json["usage"]),
        [
            "cached_tokens",
            "input_tokens",
            "output_tokens",
            "reasoning_tokens"
        ]
    );
    assert_eq!(json["usage"]["input_tokens"], 12_000);
}
