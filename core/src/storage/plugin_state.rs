//! A plugin's own document in this profile (ADR-0084). Opaque text: the store keeps it and hands it
//! back, and nothing in the app ever reads what is inside.

use super::Store;
use crate::error::Result;
use rusqlite::{OptionalExtension, params};

impl Store {
    pub fn plugin_state(&self, plugin: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT state FROM plugin_state WHERE plugin = ?1",
                [plugin],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Replaces the plugin's whole document: there is one per plugin, never a merge.
    pub fn save_plugin_state(&self, plugin: &str, state: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO plugin_state (plugin, state, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT (plugin) DO UPDATE SET state = excluded.state, updated_at = excluded.updated_at",
            params![plugin, state],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::Store;

    #[test]
    fn a_saved_document_replaces_the_last_and_stays_the_plugins_own() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(
            store.plugin_state("a").unwrap(),
            None,
            "nothing kept yet is not an empty one"
        );

        store.save_plugin_state("a", r#"{"rules":[]}"#).unwrap();
        store.save_plugin_state("a", r#"{"rules":[1]}"#).unwrap();
        store.save_plugin_state("b", "{}").unwrap();

        assert_eq!(
            store.plugin_state("a").unwrap().as_deref(),
            Some(r#"{"rules":[1]}"#)
        );
        assert_eq!(store.plugin_state("b").unwrap().as_deref(), Some("{}"));
    }
}
