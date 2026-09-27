//! AI assistant: keys, chat, tools. Host-only; nothing here but `commands/ai/` names Tauri
//! (`.claude/rules/ai-assistant.md`).

pub mod anthropic;
pub mod brief;
pub mod catalog;
pub mod compat;
pub mod consent;
pub mod export;
pub mod fake;
pub mod gemini;
pub mod guide;
pub mod keys;
pub mod models;
pub mod openai;
pub mod session;
pub mod sse;
pub mod store;
pub mod tools;

use serde::{Deserialize, Serialize};

/// Neutral across providers: OpenAI's `assistant`, Anthropic's `assistant`, Gemini's `model` all
/// map to `Model` at the adapter boundary, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Model,
}

/// A turn's content, provider-agnostic; also the stored `ai_messages.content` shape (ADR-0037).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    // A struct variant, not `Text(String)`: serde's internal tagging (`tag = "type"`) cannot
    // merge a tag into a bare JSON string, only into an object.
    Text {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        args_json: String,
        /// Kept only to hand back: Gemini refuses a replayed call without it.
        #[serde(default)]
        signature: Option<String>,
    },
    ToolResult {
        call_id: String,
        content: String,
    },
    /// A search the provider ran itself. Shown, never replayed.
    WebSearch {
        query: String,
    },
    /// The provider's summary of its reasoning. Shown and stored, never replayed.
    Reasoning {
        text: String,
        /// The one exception to "never replayed": Anthropic rejects a signed step without it.
        #[serde(default)]
        signature: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Refusal,
    /// The user pressed stop. Whatever text had already streamed is kept: it is what they saw.
    Cancelled,
    Error(String),
}

impl StopReason {
    /// A stop the user has to be told about. Whatever text arrived before it is still kept and
    /// shown; this is the reason the answer ends where it does.
    pub fn failure(&self) -> Option<AiError> {
        match self {
            StopReason::Refusal => Some(AiError::Refused),
            StopReason::MaxTokens => Some(AiError::Truncated),
            StopReason::Error(reason) => Some(AiError::Provider(format!("the response stopped: {reason}"))),
            StopReason::EndTurn | StopReason::ToolUse | StopReason::Cancelled => None,
        }
    }
}

/// The one cost lever every provider exposes under a different name (`reasoning.effort`,
/// `thinking`, ...); the adapter maps it, including "not supported by this model" as a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Low,
    Medium,
    High,
}

/// What a request cost, counted by the provider. The core's own shape, because this is also
/// what gets written down (`ai_usage`) — one struct rather than one per layer.
pub use sq_core::model::AiUsage as Usage;

/// One request to a model. `system` is a field of its own, not a message — none of OpenAI,
/// Anthropic or Gemini put it in the history, each under its own field name.
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub system: String,
    /// The moving part of the prompt (date, screen, lens), rendered after the fixed text so the
    /// cached prefix still matches (ADR-0037).
    pub context: String,
    pub model: String,
    pub effort: Effort,
    /// Turns in order; each is one role and the blocks it produced or was given.
    pub messages: Vec<(Role, Vec<Block>)>,
    /// Name, description and JSON Schema per tool. Fixed for the whole chat: changing the set
    /// mid-conversation invalidates the provider's prefix cache (ADR-0037).
    pub tools: Vec<ToolDef>,
    /// Whether the provider may search the web itself. Not one of `tools`: it is executed by the
    /// provider, not by us, so it has no schema and no body here.
    pub web_search: bool,
    /// Off by default: it costs tokens and some accounts are not cleared for it.
    pub reasoning_summary: bool,
    /// Ceiling on everything the model writes — reasoning included, which every provider counts
    /// as output. `None` is `DEFAULT_MAX_OUTPUT`; only the dashboard brief narrows it.
    pub max_output: Option<u32>,
}

/// What a request may write when nobody asked for less: high enough that no answer a chat
/// produces is cut off by it, so a `truncated` stop means the model really ran away.
pub const DEFAULT_MAX_OUTPUT: u32 = 64_000;

/// One tool as a provider-neutral declaration; the adapter shapes it for its own API.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

/// No failure variant: a failure is the call's `Err`, worded by the frontend from a code.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    /// A chunk of the model's text, as it streams. A struct variant, not `Text(String)`: serde's
    /// internal tagging cannot merge a tag into a bare JSON string.
    Text { text: String },
    /// Blocks the loop until `ai_tool_decide` answers `request_id`; `params` are values, not a sentence.
    ToolRequested {
        request_id: String,
        tool: String,
        params: tools::Params,
        /// The model's reason, shown beside the host's values, never instead of them.
        reason: String,
        /// A write is confirmed every time, so the card offers no standing permission (ADR-0037).
        write: bool,
    },
    /// A tool is running, so the panel can say what it is waiting on instead of going quiet.
    /// It carries the same `reason`, so a call that ran without asking still says why.
    ToolRunning { tool: String, reason: String },
    /// A tool finished; carries what it answered so the panel can show the reading as it lands
    /// rather than only once the whole turn is persisted.
    ToolFinished { tool: String, content: String },
    /// The provider is searching the web on its own behalf.
    Searching { query: String },
    /// A chunk of the reasoning summary, as it streams. Only arrives when the request asked for
    /// one; a model that summarises nothing simply never sends this.
    Reasoning { text: String },
    /// Cumulative over the turn, so a missed event still leaves the right total.
    Usage { usage: Usage },
    /// The turn is over *and persisted* — a listener may reread the chat on this and see it.
    Done,
}

/// What a completed turn produced.
#[derive(Debug, Clone)]
pub struct AiTurn {
    pub blocks: Vec<Block>,
    pub stop_reason: StopReason,
    pub usage: Usage,
}

/// Streaming only. `cancelled` is polled between wire events, so stopping ends the reading.
pub trait AiProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn>;
}

#[derive(Debug, Clone)]
pub enum AiError {
    /// The key is missing or the provider rejected it (401).
    Auth(String),
    /// The provider is throttling (429); `retry_after` from the header, when it sent one.
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },
    /// Reached the network, got back something that was not a usable response.
    Network(String),
    /// The provider answered with a real error (`response.failed`, a non-2xx JSON error body).
    Provider(String),
    /// The model declined to answer: a safety stop, or a refusal in place of the text.
    Refused,
    /// The answer reached the output limit before it was finished.
    Truncated,
    /// Encoding/decoding a request or the stored chat history failed.
    Storage(String),
    /// A tool could not answer: a bad argument, or data the portfolio does not have. Reported
    /// back *to the model* as a tool result, not raised to the user — it is the model's to fix.
    Tool(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::Auth(m) => write!(f, "{m}"),
            AiError::RateLimit { message, .. } => write!(f, "{message}"),
            AiError::Network(m) => write!(f, "{m}"),
            AiError::Provider(m) => write!(f, "{m}"),
            AiError::Refused => write!(f, "the model declined to answer"),
            AiError::Truncated => write!(f, "the answer reached the output limit"),
            AiError::Storage(m) => write!(f, "{m}"),
            AiError::Tool(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AiError {}

pub type AiResult<T> = std::result::Result<T, AiError>;

/// The provider's `error.message`; the raw body only as a fallback.
pub(crate) fn http_error(status: u16, body: &str) -> AiError {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let message = parsed["error"]["message"]
        .as_str()
        .or_else(|| parsed["error"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().chars().take(300).collect());
    AiError::Provider(format!("http {status}: {message}"))
}
