//! Chat history as neutral turns: `Role`/`Block` in, `Role`/`Block` out. `sq_core::Store` keeps
//! `role` and `content` as opaque strings (see the migration's own comment), and this file is the
//! one place that knows what those strings mean — the agentic loop and the commands both read
//! through it rather than each decoding JSON of their own.
//!
//! Knows nothing about Tauri, like the rest of `ai/` outside `keys.rs`.

use super::{AiError, AiResult, Block, Role};
use serde::Serialize;
use sq_core::storage::Store;

/// One stored turn with its blocks already decoded. Also the wire shape `ai_messages_list`
/// returns: the frontend never parses a string field of its own.
#[derive(Debug, Clone, Serialize)]
pub struct ChatTurn {
    pub id: String,
    pub chat_id: String,
    pub role: Role,
    pub blocks: Vec<Block>,
    pub created_at: String,
}

/// Every turn of a chat, oldest first.
pub fn turns(store: &Store, chat_id: &str) -> AiResult<Vec<ChatTurn>> {
    store
        .ai_messages_list(chat_id)
        .map_err(storage)?
        .into_iter()
        .map(|m| {
            Ok(ChatTurn {
                id: m.id,
                chat_id: m.chat_id,
                role: role_from_stored(&m.role),
                blocks: decode(&m.content)?,
                created_at: m.created_at,
            })
        })
        .collect()
}

/// The same rows in the shape [`super::AiRequest::messages`] takes.
pub fn history(store: &Store, chat_id: &str) -> AiResult<Vec<(Role, Vec<Block>)>> {
    Ok(turns(store, chat_id)?
        .into_iter()
        .map(|t| (t.role, t.blocks))
        .collect())
}

pub fn append(store: &Store, chat_id: &str, role: Role, blocks: &[Block]) -> AiResult<()> {
    store
        .ai_message_append(chat_id, stored_role(role), &encode(blocks)?)
        .map_err(storage)?;
    Ok(())
}

/// The stored spelling is the wire spelling — `Role`'s own serde renaming produces these two
/// strings, so a chat exported and re-read never has to translate between them.
fn stored_role(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Model => "model",
    }
}

/// Anything that is not the model is the user: a row written by a version that spelled the role
/// differently should still render as a side of the conversation, not fail the whole chat.
fn role_from_stored(role: &str) -> Role {
    if role == "model" { Role::Model } else { Role::User }
}

fn encode(blocks: &[Block]) -> AiResult<String> {
    serde_json::to_string(blocks).map_err(|e| AiError::Storage(e.to_string()))
}

fn decode(content: &str) -> AiResult<Vec<Block>> {
    serde_json::from_str(content).map_err(|e| AiError::Storage(e.to_string()))
}

fn storage(e: sq_core::Error) -> AiError {
    AiError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_round_trips_through_the_store_with_its_blocks_intact() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();

        let blocks = vec![
            Block::Text { text: "hi".into() },
            Block::ToolCall {
                id: "call_1".into(),
                name: "portfolio_overview".into(),
                args_json: "{}".into(),
                signature: None,
            },
        ];
        append(&store, &chat.id, Role::Model, &blocks).unwrap();

        let turns = turns(&store, &chat.id).unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].role, Role::Model);
        assert_eq!(turns[0].blocks, blocks);
    }

    #[test]
    fn the_stored_role_and_the_wire_role_are_the_same_two_strings() {
        for role in [Role::User, Role::Model] {
            let wire = serde_json::to_string(&role).unwrap();
            assert_eq!(wire, format!("\"{}\"", stored_role(role)));
            assert_eq!(role_from_stored(stored_role(role)), role);
        }
    }
}
