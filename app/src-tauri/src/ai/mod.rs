//! AI assistant: provider keys, chat, tools. `core` never knows this exists
//! (see `.claude/rules/ai-assistant.md`) — network, secrets and user consent stay in the host.
//!
//! `mod.rs`, the provider adapters and `sse.rs` know nothing about Tauri — `grep -rn "tauri"`
//! over them must stay empty, the same discipline `ui-boundary.md` holds `core` to.

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

/// A turn's content, provider-agnostic. This is also the shape stored in `ai_messages.content`
/// (as JSON) — fixed now, deliberately ahead of a second provider, because the cost of a wrong
/// guess here is incompatible data in a user's chat history, not a code rewrite. See
/// ADR-0037.
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
        /// What the provider signed this step with, when it signs them. Kept only to be handed
        /// back: Gemini refuses a follow-up whose call arrived without its signature, and a
        /// provider that signs nothing (OpenAI, and older stored rows) leaves it absent.
        #[serde(default)]
        signature: Option<String>,
    },
    ToolResult {
        call_id: String,
        content: String,
    },
    /// A search the *provider* ran on its own — OpenAI's hosted `web_search`, and the same idea
    /// under another name at Anthropic and Gemini. There is nothing to hand back: the provider
    /// already has the results, and this block only records that it happened, so the panel can
    /// show it and the user can see the question left the machine.
    WebSearch {
        query: String,
    },
    /// The model's *summary* of its own reasoning, not the reasoning itself — no provider hands
    /// the raw chain over. Shown and stored, never replayed: the thinking belongs to the step
    /// that produced it, and a summary fed back is text the model would have to read as if
    /// somebody else wrote it.
    Reasoning {
        text: String,
        /// The provider's proof that this summary is its own, when it issues one. The single
        /// exception to "never replayed": Anthropic rejects a signed step handed back without
        /// it. Absent everywhere else, including in rows written before this existed.
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
    /// What is true of this moment — the date, the screen, the lens. Kept apart from `system`
    /// because it changes between requests and the static part must not: the adapter renders it
    /// *after* the fixed text, so everything before it still matches the provider's cached
    /// prefix (ADR-0037).
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
    /// Whether to ask for a summary of the model's reasoning. Off is the safe default: the
    /// summary costs output tokens, and some provider accounts are not cleared to receive one
    /// at all — a chat must not stop working because the app asked for something extra.
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

/// Streamed as tokens arrive. Growing this (e.g. a `ToolRequested` variant) waits for step 3,
/// when there is a consumer for it — unlike the storage shape above, this one is not the
/// documented exception to "build it after the second example".
///
/// There is deliberately no failure variant: a failure is the `Err` of the call, and the sentence
/// for it is written by the frontend from a code (`commands::ai::AiStreamEvent`). An event
/// carrying an English message would be text crossing IPC — see `.claude/rules/ui-boundary.md`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    /// A chunk of the model's text, as it streams. A struct variant, not `Text(String)`: serde's
    /// internal tagging cannot merge a tag into a bare JSON string.
    Text { text: String },
    /// The model wants a tool the user has not allowed in this chat. The loop is blocked until
    /// `ai_tool_decide` answers this `request_id` — see `consent.rs` for why the id is the whole
    /// message. `params` are values, not a sentence: the frontend writes the card.
    ToolRequested {
        request_id: String,
        tool: String,
        params: tools::Params,
        /// The model's own one line on why it wants this, in the user's language. Shown *beside*
        /// the values, never instead of them: the facts on the card are the host's, and this is
        /// an intention, which is exactly the thing a card must not be made of on its own.
        reason: String,
        /// Whether this call *changes* the portfolio. The card's wording and its buttons differ:
        /// a write is confirmed every time, so offering "allow in this chat" for one would be
        /// offering something the host will never honour (ADR-0037).
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
    /// What this turn has cost so far, as the provider counted it — cumulative over the steps
    /// of one turn, not a delta, so a listener that missed one still shows the right total.
    /// Providers report it at different moments (OpenAI only with the finished response,
    /// Anthropic as it streams), which is why this is an event rather than a field on `AiTurn`.
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

/// One request to a model. Streaming is the only mode: a chat without tokens arriving is a chat
/// that looks frozen for twenty seconds.
///
/// `cancelled` is polled between wire events rather than checked afterwards: stopping has to end
/// the *reading*, and a reply already paid for should still be shown.
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

/// A non-2xx answer as the provider's own line. All four wires nest it at `error.message`
/// (a gateway may put a bare string at `error`); the raw body is the fallback, never the norm —
/// a JSON dump is not something to show a user.
pub(crate) fn http_error(status: u16, body: &str) -> AiError {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let message = parsed["error"]["message"]
        .as_str()
        .or_else(|| parsed["error"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().chars().take(300).collect());
    AiError::Provider(format!("http {status}: {message}"))
}
