//! What the assistant's requests cost. A row per request, written by the host after the provider
//! reports it; nothing here calls a provider or knows one exists.

use super::Store;
use crate::error::Result;
use crate::model::{AiUsage, AiUsageTotal};
use rusqlite::{Row, params};

impl Store {
    /// Records one request. `chat_id` is absent for a reading with no conversation behind it —
    /// the dashboard brief.
    pub fn ai_usage_record(
        &self,
        chat_id: Option<&str>,
        provider: &str,
        model: &str,
        usage: AiUsage,
    ) -> Result<()> {
        let id = crate::model::new_id();
        self.conn.execute(
            "INSERT INTO ai_usage (id, chat_id, provider, model, input_tokens, cached_tokens,
                                   output_tokens, reasoning_tokens, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
            params![
                id,
                chat_id,
                provider,
                model,
                usage.input_tokens,
                usage.cached_tokens,
                usage.output_tokens,
                usage.reasoning_tokens,
            ],
        )?;
        Ok(())
    }

    /// Everything ever spent, by model, heaviest first. Rows of a deleted chat still count: the
    /// question this answers is what the key was billed for, not which conversations exist.
    pub fn ai_usage_totals(&self) -> Result<Vec<AiUsageTotal>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, model, COUNT(*), SUM(input_tokens), SUM(cached_tokens),
                    SUM(output_tokens), SUM(reasoning_tokens), MIN(created_at), MAX(created_at)
             FROM ai_usage
             GROUP BY provider, model
             ORDER BY SUM(input_tokens) + SUM(output_tokens) DESC",
        )?;
        let rows = stmt
            .query_map([], row_to_total)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// What one chat has cost so far, across every request it made.
    pub fn ai_usage_for_chat(&self, chat_id: &str) -> Result<AiUsage> {
        let usage = self.conn.query_row(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(cached_tokens), 0),
                    COALESCE(SUM(output_tokens), 0), COALESCE(SUM(reasoning_tokens), 0)
             FROM ai_usage WHERE chat_id = ?1",
            [chat_id],
            |row| {
                Ok(AiUsage {
                    input_tokens: row.get(0)?,
                    cached_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                    reasoning_tokens: row.get(3)?,
                })
            },
        )?;
        Ok(usage)
    }
}

fn row_to_total(row: &Row) -> rusqlite::Result<AiUsageTotal> {
    Ok(AiUsageTotal {
        provider: row.get(0)?,
        model: row.get(1)?,
        requests: row.get(2)?,
        usage: AiUsage {
            input_tokens: row.get(3)?,
            cached_tokens: row.get(4)?,
            output_tokens: row.get(5)?,
            reasoning_tokens: row.get(6)?,
        },
        first_at: row.get(7)?,
        last_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(input: i64, output: i64) -> AiUsage {
        AiUsage {
            input_tokens: input,
            cached_tokens: 0,
            output_tokens: output,
            reasoning_tokens: 0,
        }
    }

    #[test]
    fn totals_add_up_per_model_and_count_requests_not_turns() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();

        store
            .ai_usage_record(Some(&chat.id), "openai", "gpt-5.1", usage(1_000, 200))
            .unwrap();
        store
            .ai_usage_record(Some(&chat.id), "openai", "gpt-5.1", usage(1_400, 50))
            .unwrap();
        store
            .ai_usage_record(None, "openai", "gpt-5.1-mini", usage(300, 90))
            .unwrap();

        let totals = store.ai_usage_totals().unwrap();
        assert_eq!(totals.len(), 2);
        // 1000 + 200 + 1400 + 50 = 2650 against 300 + 90 = 390, so the flagship sorts first.
        assert_eq!(totals[0].model, "gpt-5.1");
        assert_eq!(totals[0].requests, 2);
        assert_eq!(totals[0].usage.input_tokens, 2_400);
        assert_eq!(totals[0].usage.output_tokens, 250);
        assert_eq!(totals[1].model, "gpt-5.1-mini");

        assert_eq!(store.ai_usage_for_chat(&chat.id).unwrap(), usage(2_400, 250));
    }

    #[test]
    fn deleting_a_chat_keeps_what_it_spent() {
        let store = Store::open_in_memory().unwrap();
        let chat = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap();
        store
            .ai_usage_record(Some(&chat.id), "openai", "gpt-5.1", usage(500, 100))
            .unwrap();

        store.ai_chat_delete(&chat.id).unwrap();

        let totals = store.ai_usage_totals().unwrap();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].requests, 1);
        assert_eq!(totals[0].usage.input_tokens, 500);
        // The chat is gone, so nothing is attributed to it any more.
        assert_eq!(store.ai_usage_for_chat(&chat.id).unwrap(), AiUsage::default());
    }
}
