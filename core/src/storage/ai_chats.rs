use super::Store;
use crate::error::{Error, Result};
use crate::model::{AiChat, AiEffort, AiMessage, AiToolMode};
use rusqlite::params;

impl Store {
    /// Creates a chat; timestamps are the database clock, not the caller's.
    pub fn ai_chat_create(&self, title: &str, provider: &str, model: &str) -> Result<AiChat> {
        let id = crate::model::new_id();
        self.conn.execute(
            "INSERT INTO ai_chats (id, title, provider, model, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))",
            params![id, title, provider, model],
        )?;
        self.ai_chat_get(&id)
    }

    pub fn ai_chat_get(&self, id: &str) -> Result<AiChat> {
        self.conn
            .query_row(
                "SELECT id, title, provider, model, tool_mode, effort, created_at, updated_at
                 FROM ai_chats WHERE id = ?1",
                [id],
                Self::row_to_chat,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("ai chat {id}")),
                other => other.into(),
            })
    }

    /// Most recently active chat first.
    pub fn ai_chats_list(&self) -> Result<Vec<AiChat>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, provider, model, tool_mode, effort, created_at, updated_at FROM ai_chats ORDER BY updated_at DESC")?;
        let rows = stmt
            .query_map([], Self::row_to_chat)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn ai_chat_rename(&self, id: &str, title: &str) -> Result<()> {
        let touched = self
            .conn
            .execute("UPDATE ai_chats SET title = ?2 WHERE id = ?1", params![id, title])?;
        if touched == 0 {
            return Err(Error::NotFound(format!("ai chat {id}")));
        }
        Ok(())
    }

    /// Cascades to every message on the chat.
    pub fn ai_chat_delete(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM ai_chats WHERE id = ?1", [id])?;
        Ok(())
    }

    /// In the order they were written — never by `created_at`, which two turns in one exchange
    /// can share.
    pub fn ai_messages_list(&self, chat_id: &str) -> Result<Vec<AiMessage>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, chat_id, role, content, created_at FROM ai_messages WHERE chat_id = ?1 ORDER BY rowid")?;
        let rows = stmt
            .query_map([chat_id], Self::row_to_message)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Appends one turn and bumps the chat's `updated_at` in the same transaction.
    pub fn ai_message_append(&self, chat_id: &str, role: &str, content: &str) -> Result<AiMessage> {
        let id = crate::model::new_id();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO ai_messages (id, chat_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![id, chat_id, role, content],
        )?;
        let touched = tx.execute(
            "UPDATE ai_chats SET updated_at = datetime('now') WHERE id = ?1",
            [chat_id],
        )?;
        if touched == 0 {
            return Err(Error::NotFound(format!("ai chat {chat_id}")));
        }
        let created_at: String =
            tx.query_row("SELECT created_at FROM ai_messages WHERE id = ?1", [&id], |r| {
                r.get(0)
            })?;
        tx.commit()?;
        Ok(AiMessage {
            id,
            chat_id: chat_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at,
        })
    }

    /// Switches how this chat treats tool calls. Not a setting: it lives on the conversation.
    pub fn ai_chat_set_mode(&self, id: &str, mode: AiToolMode) -> Result<()> {
        self.touch_chat(id, "tool_mode", mode.as_str())
    }

    /// Sets how hard this chat asks the model to think.
    pub fn ai_chat_set_effort(&self, id: &str, effort: AiEffort) -> Result<()> {
        self.touch_chat(id, "effort", effort.as_str())
    }

    /// Switches this chat to another model. Only what a chat is *answered* with changes; the
    /// history replays into any adapter, so a chat is never tied to the model that started it.
    pub fn ai_chat_set_model(&self, id: &str, model: &str) -> Result<()> {
        self.touch_chat(id, "model", model)
    }

    /// Switches this chat to another provider. Provider and model move together because a model
    /// id belongs to the catalogue it came from: leaving one behind names a model the new
    /// provider has never heard of, and the next request 404s.
    pub fn ai_chat_set_provider(&self, id: &str, provider: &str, model: &str) -> Result<()> {
        let touched = self.conn.execute(
            "UPDATE ai_chats SET provider = ?2, model = ?3 WHERE id = ?1",
            params![id, provider, model],
        )?;
        if touched == 0 {
            return Err(Error::NotFound(format!("ai chat {id}")));
        }
        Ok(())
    }

    fn touch_chat(&self, id: &str, column: &str, value: &str) -> Result<()> {
        // The column name is never user input: it comes from the three callers above.
        let touched = self.conn.execute(
            &format!("UPDATE ai_chats SET {column} = ?2 WHERE id = ?1"),
            params![id, value],
        )?;
        if touched == 0 {
            return Err(Error::NotFound(format!("ai chat {id}")));
        }
        Ok(())
    }

    /// Records "this tool may read for the rest of this chat". Re-granting is a no-op, so the
    /// caller never has to ask whether the grant is already there.
    pub fn ai_grant_add(&self, chat_id: &str, tool: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ai_grants (chat_id, tool, created_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT (chat_id, tool) DO NOTHING",
            params![chat_id, tool],
        )?;
        Ok(())
    }

    pub fn ai_grants_list(&self, chat_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tool FROM ai_grants WHERE chat_id = ?1 ORDER BY tool")?;
        let rows = stmt
            .query_map([chat_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn row_to_chat(r: &rusqlite::Row) -> rusqlite::Result<AiChat> {
        Ok(AiChat {
            id: r.get(0)?,
            title: r.get(1)?,
            provider: r.get(2)?,
            model: r.get(3)?,
            tool_mode: AiToolMode::from_stored(&r.get::<_, String>(4)?),
            effort: AiEffort::from_stored(&r.get::<_, String>(5)?),
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        })
    }

    fn row_to_message(r: &rusqlite::Row) -> rusqlite::Result<AiMessage> {
        Ok(AiMessage {
            id: r.get(0)?,
            chat_id: r.get(1)?,
            role: r.get(2)?,
            content: r.get(3)?,
            created_at: r.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chat_lists_its_messages_in_order_and_bumps_updated_at() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        assert_eq!(chat.created_at, chat.updated_at);

        store.ai_message_append(&chat.id, "user", "[]").unwrap();
        store.ai_message_append(&chat.id, "model", "[]").unwrap();

        let messages = store.ai_messages_list(&chat.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "model");

        let reloaded = store.ai_chat_get(&chat.id).unwrap();
        assert!(reloaded.updated_at >= chat.updated_at);
    }

    #[test]
    fn a_new_chat_asks_and_the_mode_it_is_switched_to_sticks() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        assert_eq!(chat.tool_mode, AiToolMode::Ask, "the careful mode is the default");

        store.ai_chat_set_mode(&chat.id, AiToolMode::Auto).unwrap();
        assert_eq!(store.ai_chat_get(&chat.id).unwrap().tool_mode, AiToolMode::Auto);
        // A second chat is unaffected: the mode belongs to the conversation.
        let other = store.ai_chat_create("Other", "openai", "gpt-5.1").unwrap();
        assert_eq!(other.tool_mode, AiToolMode::Ask);
    }

    #[test]
    fn switching_provider_carries_the_model_with_it_and_leaves_other_chats_alone() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        let other = store.ai_chat_create("Other", "openai", "gpt-5.1").unwrap();

        store
            .ai_chat_set_provider(&chat.id, "anthropic", "claude-opus-5")
            .unwrap();

        let switched = store.ai_chat_get(&chat.id).unwrap();
        assert_eq!(switched.provider, "anthropic");
        // The old model never survives the move: it names a catalogue the new provider has not.
        assert_eq!(switched.model, "claude-opus-5");

        let untouched = store.ai_chat_get(&other.id).unwrap();
        assert_eq!(untouched.provider, "openai");
        assert_eq!(untouched.model, "gpt-5.1");
    }

    #[test]
    fn a_grant_is_per_chat_and_granting_twice_changes_nothing() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        let other = store.ai_chat_create("Other", "openai", "gpt-5.1").unwrap();

        store.ai_grant_add(&chat.id, "positions_list").unwrap();
        store.ai_grant_add(&chat.id, "positions_list").unwrap();

        assert_eq!(store.ai_grants_list(&chat.id).unwrap(), vec!["positions_list"]);
        assert!(store.ai_grants_list(&other.id).unwrap().is_empty());
    }

    #[test]
    fn deleting_a_chat_takes_its_grants_with_it() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        store.ai_grant_add(&chat.id, "positions_list").unwrap();

        store.ai_chat_delete(&chat.id).unwrap();

        assert!(store.ai_grants_list(&chat.id).unwrap().is_empty());
    }

    #[test]
    fn renaming_a_missing_chat_is_not_found() {
        let store = Store::open_in_memory().unwrap();
        assert!(matches!(
            store.ai_chat_rename("missing", "x"),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn deleting_a_chat_takes_its_messages_with_it() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        store.ai_message_append(&chat.id, "user", "[]").unwrap();

        store.ai_chat_delete(&chat.id).unwrap();

        assert!(store.ai_chats_list().unwrap().is_empty());
        assert!(store.ai_messages_list(&chat.id).unwrap().is_empty());
    }
}
