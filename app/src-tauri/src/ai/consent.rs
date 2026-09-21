//! The consent gate. A tool call the user has not allowed stops the loop, asks, and waits.
//!
//! The decision is resolved **by `request_id` out of host state**, never from what the frontend
//! sends back. The frontend replies with an id and a yes/no and nothing else — no tool name, no
//! arguments, no slice of history. Otherwise "what the user approved" and "what the host ran"
//! are two different objects, and a careful injection only has to separate them.
//!
//! Injection is not hypothetical here: instrument names, operation wording and account names all
//! came out of somebody else's CSV (`.claude/rules/import.md`) and end up inside a tool result.
//! That is why the card's text is built by the host from the tool's own `Params`, never by the
//! model — otherwise the model writes the label on the button the user is about to press.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Run it this once; nothing is remembered.
    Once,
    /// Run it, and for the rest of this chat stop asking for this tool.
    Session,
    /// Run it, and stop asking about reads in this chat at all — the same switch the mode
    /// toggle makes, offered where the question is actually being asked.
    Always,
    Deny,
}

impl Decision {
    pub fn from_wire(value: &str) -> Option<Decision> {
        match value {
            "once" => Some(Decision::Once),
            "session" => Some(Decision::Session),
            "always" => Some(Decision::Always),
            "deny" => Some(Decision::Deny),
            _ => None,
        }
    }
}

/// Requests waiting for an answer, keyed by the id the frontend was given.
#[derive(Default)]
pub struct Pending {
    waiting: Mutex<HashMap<String, Sender<Decision>>>,
}

impl Pending {
    /// Registers a request and hands back the end to block on.
    pub fn open(&self, request_id: &str) -> Receiver<Decision> {
        let (sender, receiver) = channel();
        self.lock().insert(request_id.to_string(), sender);
        receiver
    }

    /// Answers a waiting request. `false` means nobody was waiting under that id — a stale card,
    /// a second click, or a reply to a turn that has already been cancelled.
    pub fn decide(&self, request_id: &str, decision: Decision) -> bool {
        match self.lock().remove(request_id) {
            Some(sender) => sender.send(decision).is_ok(),
            None => false,
        }
    }

    pub fn forget(&self, request_id: &str) {
        self.lock().remove(request_id);
    }

    /// A cancelled or abandoned turn must not leave cards that can never be answered.
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// A poisoned lock here would only mean a panicking turn, and refusing to ask again would
    /// wedge the panel — the registry holds no invariant worth failing over.
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Sender<Decision>>> {
        self.waiting.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// How the loop asks. Implemented by the host (ask the user) and by tests (answer from a script).
pub trait ConsentGate {
    /// `reason` is what the model says the call is for, in the user's language. The card shows
    /// it as the model's words; the change itself is still described by the host from `params`.
    fn ask(
        &self,
        request_id: &str,
        tool: &str,
        params: &super::tools::Params,
        write: bool,
        reason: &str,
    ) -> Decision;
}

/// Blocks on `Pending` while checking for cancellation, so "stop" is answered even though nobody
/// ever pressed a button on the card.
pub fn wait(
    pending: &Pending,
    request_id: &str,
    receiver: Receiver<Decision>,
    cancelled: &dyn Fn() -> bool,
) -> Decision {
    loop {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(decision) => return decision,
            Err(RecvTimeoutError::Timeout) => {
                if cancelled() {
                    pending.forget(request_id);
                    return Decision::Deny;
                }
            }
            // The sender is gone: the request was cleared out from under us.
            Err(RecvTimeoutError::Disconnected) => return Decision::Deny,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decision_reaches_the_waiting_request_and_only_that_one() {
        let pending = Pending::default();
        let receiver = pending.open("req-1");
        let other = pending.open("req-2");

        assert!(pending.decide("req-1", Decision::Session));

        assert_eq!(wait(&pending, "req-1", receiver, &|| false), Decision::Session);
        assert!(other.try_recv().is_err(), "the other request is untouched");
    }

    #[test]
    fn an_unknown_request_id_answers_nothing() {
        let pending = Pending::default();
        assert!(!pending.decide("never-asked", Decision::Once));
    }

    #[test]
    fn deciding_twice_answers_once() {
        let pending = Pending::default();
        let _receiver = pending.open("req-1");
        assert!(pending.decide("req-1", Decision::Once));
        assert!(!pending.decide("req-1", Decision::Once));
    }

    #[test]
    fn a_cancelled_turn_stops_waiting_and_counts_as_a_refusal() {
        let pending = Pending::default();
        let receiver = pending.open("req-1");
        assert_eq!(wait(&pending, "req-1", receiver, &|| true), Decision::Deny);
        // The card is gone, so a late click cannot resurrect the call.
        assert!(!pending.decide("req-1", Decision::Once));
    }
}
