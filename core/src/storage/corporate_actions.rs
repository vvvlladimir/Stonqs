use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::Result;
use crate::model::{CorporateAction, CorporateActionKind};
use rusqlite::{Row, params};

fn row_to_action(row: &Row<'_>) -> rusqlite::Result<CorporateAction> {
    let kind: String = row.get("kind")?;
    let date: String = row.get("date")?;
    Ok(CorporateAction {
        id: row.get("id")?,
        security_id: row.get("security_id")?,
        date: date_from_sql(&date)?,
        kind: CorporateActionKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        ratio_from: row.get::<_, SqlDecimal>("ratio_from")?.0,
        ratio_to: row.get::<_, SqlDecimal>("ratio_to")?.0,
        note: row.get("note")?,
    })
}

impl Store {
    pub fn save_corporate_action(&self, a: &CorporateAction) -> Result<()> {
        a.validate()?;
        self.conn.execute(
            "INSERT INTO corporate_actions (id, security_id, date, kind, ratio_from, ratio_to, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET
                 security_id = excluded.security_id,
                 date = excluded.date,
                 kind = excluded.kind,
                 ratio_from = excluded.ratio_from,
                 ratio_to = excluded.ratio_to,
                 note = excluded.note",
            params![
                a.id,
                a.security_id,
                date_to_sql(a.date),
                a.kind.as_str(),
                dec_to_sql(a.ratio_from),
                dec_to_sql(a.ratio_to),
                a.note,
            ],
        )?;
        Ok(())
    }

    pub fn delete_corporate_action(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM corporate_actions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Returns all corporate actions in chronological order.
    pub fn list_corporate_actions(&self) -> Result<Vec<CorporateAction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM corporate_actions ORDER BY date, id")?;
        let rows = stmt.query_map([], row_to_action)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn corporate_actions_for_security(&self, security_id: &str) -> Result<Vec<CorporateAction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM corporate_actions WHERE security_id = ?1 ORDER BY date, id")?;
        let rows = stmt.query_map([security_id], row_to_action)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
