//! A provider that answers from a script instead of the network, so the agentic loop, the
//! consent gate and the storage round-trip are all testable offline — the same role
//! `StaticFxProvider` plays for `fx/` in the core.

use super::{AiEvent, AiProvider, AiRequest, AiResult, AiTurn, Block, StopReason, Usage};
use std::cell::RefCell;

pub struct StaticAiProvider {
    /// Answered in order; once the script runs out the last entry repeats, which is what makes
    /// "a model that only ever calls tools" expressible as one turn.
    script: Vec<AiResult<AiTurn>>,
    next: RefCell<usize>,
    /// Every request the double was handed, so a test can assert on what the loop built rather
    /// than only on what came back.
    pub seen: RefCell<Vec<AiRequest>>,
}

impl StaticAiProvider {
    /// One plain text reply, repeated.
    pub fn text(reply: impl Into<String>) -> Self {
        Self::turn(AiTurn {
            blocks: vec![Block::Text { text: reply.into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        })
    }

    /// One turn, repeated for as many steps as the loop takes.
    pub fn turn(turn: AiTurn) -> Self {
        Self::script(vec![Ok(turn)])
    }

    pub fn failing(error: super::AiError) -> Self {
        Self::script(vec![Err(error)])
    }

    pub fn script(script: Vec<AiResult<AiTurn>>) -> Self {
        StaticAiProvider {
            script,
            next: RefCell::new(0),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl AiProvider for StaticAiProvider {
    fn stream(
        &self,
        request: &AiRequest,
        sink: &mut dyn FnMut(AiEvent),
        _cancelled: &dyn Fn() -> bool,
    ) -> AiResult<AiTurn> {
        self.seen.borrow_mut().push(request.clone());

        let index = (*self.next.borrow()).min(self.script.len() - 1);
        *self.next.borrow_mut() = index + 1;
        let turn = self.script[index].clone()?;

        // Streamed whole rather than chunked: what a caller can assert about deltas is that they
        // concatenate to the turn's text, and one chunk satisfies that as well as ten.
        for block in &turn.blocks {
            if let Block::Text { text } = block {
                sink(AiEvent::Text { text: text.clone() });
            }
        }
        Ok(turn)
    }
}
