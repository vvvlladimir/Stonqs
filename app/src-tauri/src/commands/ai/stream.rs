//! What travels over the `ai_send` / `ai_brief` channel.

use crate::ai::AiEvent;
use crate::ai::consent::{ConsentGate, Decision, Pending};
use crate::ai::tools::Params;
use crate::error::UiError;
use serde::Serialize;
use sq_core::model::AiUsage;
use tauri::ipc::Channel;

/// A failed turn arrives as `Error { UiError }`, a code the frontend words; `AiEvent` has no
/// failure variant of its own.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiStreamEvent {
    Text {
        text: String,
    },
    /// The turn blocks on this card until `ai_tool_decide` answers `request_id`.
    ToolRequested {
        request_id: String,
        tool: String,
        params: Params,
        write: bool,
        /// The model's one line on why, shown beside the values, never instead of them.
        reason: String,
    },
    ToolRunning {
        tool: String,
        reason: String,
    },
    ToolFinished {
        tool: String,
        content: String,
    },
    Searching {
        query: String,
    },
    /// A chunk of the model's summary of its reasoning.
    Reasoning {
        text: String,
    },
    /// The turn's running total: the panel assigns it, never adds it up.
    Usage {
        usage: AiUsage,
    },
    Done,
    Error {
        error: UiError,
    },
}

impl From<AiEvent> for AiStreamEvent {
    fn from(event: AiEvent) -> Self {
        match event {
            AiEvent::Text { text } => AiStreamEvent::Text { text },
            AiEvent::ToolRequested {
                request_id,
                tool,
                params,
                write,
                reason,
            } => AiStreamEvent::ToolRequested {
                request_id,
                tool,
                params,
                write,
                reason,
            },
            AiEvent::ToolRunning { tool, reason } => AiStreamEvent::ToolRunning { tool, reason },
            AiEvent::ToolFinished { tool, content } => AiStreamEvent::ToolFinished { tool, content },
            AiEvent::Searching { query } => AiStreamEvent::Searching { query },
            AiEvent::Reasoning { text } => AiStreamEvent::Reasoning { text },
            AiEvent::Usage { usage } => AiStreamEvent::Usage { usage },
            AiEvent::Done => AiStreamEvent::Done,
        }
    }
}

/// The consent gate as the panel sees it: draw a card, block, let `ai_tool_decide` answer.
pub(super) struct PanelGate<'a> {
    pub pending: &'a Pending,
    pub channel: &'a Channel<AiStreamEvent>,
    pub cancelled: &'a dyn Fn() -> bool,
}

impl ConsentGate for PanelGate<'_> {
    fn ask(&self, request_id: &str, tool: &str, params: &Params, write: bool, reason: &str) -> Decision {
        let receiver = self.pending.open(request_id);
        let sent = self.channel.send(AiStreamEvent::ToolRequested {
            request_id: request_id.to_string(),
            tool: tool.to_string(),
            params: params.clone(),
            write,
            reason: reason.to_string(),
        });
        if sent.is_err() {
            // The window went away mid-turn: refuse rather than wait for a card nobody sees.
            self.pending.forget(request_id);
            return Decision::Deny;
        }
        crate::ai::consent::wait(self.pending, request_id, receiver, self.cancelled)
    }
}
