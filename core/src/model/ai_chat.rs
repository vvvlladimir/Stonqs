use serde::{Deserialize, Serialize};

/// How one chat treats tool calls. A property of the conversation, not of the app: a chat
/// opened in [`AiToolMode::Auto`] to explore must not make the next one permissive too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiToolMode {
    /// Every tool not already granted is put in front of the user.
    #[default]
    Ask,
    /// Read tools run without asking, for this chat only.
    Auto,
}

impl AiToolMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AiToolMode::Ask => "ASK",
            AiToolMode::Auto => "AUTO",
        }
    }

    /// Anything unrecognised is the careful answer, never the permissive one.
    pub fn from_stored(value: &str) -> Self {
        match value {
            "AUTO" => AiToolMode::Auto,
            _ => AiToolMode::Ask,
        }
    }
}

/// How hard the model is asked to think. Every provider spells this differently (OpenAI
/// `reasoning.effort`, Anthropic `thinking`, Gemini `thinkingConfig`) and some models support
/// none of it — the adapter decides what to make of it, including nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiEffort {
    Low,
    #[default]
    Medium,
    High,
}

impl AiEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            AiEffort::Low => "LOW",
            AiEffort::Medium => "MEDIUM",
            AiEffort::High => "HIGH",
        }
    }

    pub fn from_stored(value: &str) -> Self {
        match value {
            "LOW" => AiEffort::Low,
            "HIGH" => AiEffort::High,
            _ => AiEffort::Medium,
        }
    }
}

/// One AI assistant conversation. `title`/`provider`/`model` are the chat's own; the messages
/// that make it up are [`AiMessage`] rows, read separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiChat {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub tool_mode: AiToolMode,
    pub effort: AiEffort,
    pub created_at: String,
    pub updated_at: String,
}

/// One turn. `role` is `"user"` or `"model"`. `content` is the host's own JSON encoding of that
/// turn's blocks (text, tool calls, tool results) — opaque here, see the migration's own comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiMessage {
    pub id: String,
    pub chat_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// What one request to a model cost, as the provider counted it. Every adapter fills the same
/// four figures and reports 0 for one its provider does not count — a neutral shape the way
/// [`AiToolMode`] is, so a second provider adds no column.
///
/// `cached_tokens` is part of `input_tokens`, not beside it: it says how much of the prompt the
/// provider served from its own cache, which is the number that says whether the fixed prefix
/// still matches (`.claude/rules/ai-assistant.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiUsage {
    pub input_tokens: i64,
    pub cached_tokens: i64,
    pub output_tokens: i64,
    /// Thinking the model was billed for but never showed. Part of `output_tokens`.
    pub reasoning_tokens: i64,
}

impl AiUsage {
    pub fn plus(self, other: AiUsage) -> AiUsage {
        AiUsage {
            input_tokens: self.input_tokens + other.input_tokens,
            cached_tokens: self.cached_tokens + other.cached_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            reasoning_tokens: self.reasoning_tokens + other.reasoning_tokens,
        }
    }

    pub fn is_zero(self) -> bool {
        self == AiUsage::default()
    }
}

/// Everything spent on one model, rolled up. Per model rather than per chat because that is the
/// axis prices are quoted on — a chat is deleted, the spending it caused is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiUsageTotal {
    pub provider: String,
    pub model: String,
    /// Requests, not turns: one answer that called tools cost several.
    pub requests: i64,
    pub usage: AiUsage,
    pub first_at: String,
    pub last_at: String,
}
