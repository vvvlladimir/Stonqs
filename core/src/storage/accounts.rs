use super::{Store, date_from_sql, date_to_sql};
use crate::error::{Error, Result};
use crate::model::{Account, AccountKind};
use rusqlite::{Row, params};

fn row_to_account(row: &Row<'_>) -> rusqlite::Result<Account> {
    let kind: String = row.get("kind")?;
    let opened: Option<String> = row.get("opened_at")?;
    Ok(Account {
        id: row.get("id")?,
        name: row.get("name")?,
        currency: row.get("currency")?,
        kind: AccountKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        reference_account_id: row.get("reference_account_id")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        opened_at: opened.as_deref().map(date_from_sql).transpose()?,
    })
}

impl Store {
    /// Inserts or updates an account by `id`.
    pub fn save_account(&self, a: &Account) -> Result<()> {
        a.validate()?;
        if let Some(reference) = &a.reference_account_id {
            let target = self.get_account(reference)?;
            if target.kind != AccountKind::Deposit {
                return Err(Error::Invalid(format!(
                    "account {:?} is not a cash account: a securities account cannot settle through another one",
                    target.name
                )));
            }
        }
        self.conn.execute(
            "INSERT INTO accounts (id, name, currency, kind, is_active, opened_at, reference_account_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 currency = excluded.currency,
                 kind = excluded.kind,
                 is_active = excluded.is_active,
                 opened_at = excluded.opened_at,
                 reference_account_id = excluded.reference_account_id",
            params![
                a.id,
                a.name,
                a.currency,
                a.kind.as_str(),
                a.is_active as i64,
                a.opened_at.map(date_to_sql),
                a.reference_account_id,
            ],
        )?;
        Ok(())
    }

    /// Returns securities accounts referencing a deposit account.
    pub fn accounts_referencing(&self, deposit_id: &str) -> Result<Vec<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM accounts WHERE reference_account_id = ?1 ORDER BY name")?;
        let rows = stmt.query_map([deposit_id], row_to_account)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_account(&self, id: &str) -> Result<Account> {
        self.conn
            .query_row("SELECT * FROM accounts WHERE id = ?1", [id], row_to_account)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("account {id}")),
                other => other.into(),
            })
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self.conn.prepare("SELECT * FROM accounts ORDER BY name")?;
        let rows = stmt.query_map([], row_to_account)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
        self.forget_cash_subjects(id)?;
        Ok(())
    }
}
