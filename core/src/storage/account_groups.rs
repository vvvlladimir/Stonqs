use super::Store;
use crate::error::{Error, Result};
use crate::model::AccountGroup;
use rusqlite::params;

impl Store {
    /// Saves an account group and replaces its membership atomically.
    pub fn save_account_group(&self, g: &AccountGroup) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO account_groups (id, name) VALUES (?1, ?2)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name",
            params![g.id, g.name],
        )?;
        tx.execute("DELETE FROM account_group_members WHERE group_id = ?1", [&g.id])?;
        {
            let mut stmt =
                tx.prepare("INSERT INTO account_group_members (group_id, account_id) VALUES (?1, ?2)")?;
            for account_id in &g.account_ids {
                stmt.execute(params![g.id, account_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_account_group(&self, id: &str) -> Result<AccountGroup> {
        let name: String = self
            .conn
            .query_row("SELECT name FROM account_groups WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("account group {id}")),
                other => Error::from(other),
            })?;
        Ok(AccountGroup {
            id: id.to_string(),
            name,
            account_ids: self.account_group_members(id)?,
        })
    }

    pub fn list_account_groups(&self) -> Result<Vec<AccountGroup>> {
        let heads = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, name FROM account_groups ORDER BY name")?;
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        heads
            .into_iter()
            .map(|(id, name)| {
                let account_ids = self.account_group_members(&id)?;
                Ok(AccountGroup {
                    id,
                    name,
                    account_ids,
                })
            })
            .collect()
    }

    pub fn delete_account_group(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM account_groups WHERE id = ?1", [id])?;
        Ok(())
    }

    fn account_group_members(&self, id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.account_id FROM account_group_members m
             JOIN accounts a ON a.id = m.account_id
             WHERE m.group_id = ?1 ORDER BY a.name",
        )?;
        Ok(stmt
            .query_map([id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
