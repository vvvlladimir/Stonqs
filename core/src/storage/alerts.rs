use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{AlertCrossing, AlertDirection, AlertKind, AlertSide, CrossingDirection, SecurityAlert};
use chrono::NaiveDate;
use rusqlite::{Row, params};

fn conversion(e: Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

fn optional_date(row: &Row<'_>, column: &str) -> rusqlite::Result<Option<NaiveDate>> {
    let raw: Option<String> = row.get(column)?;
    raw.as_deref().map(date_from_sql).transpose()
}

fn row_to_alert(row: &Row<'_>) -> rusqlite::Result<SecurityAlert> {
    let kind: String = row.get("kind")?;
    let created: String = row.get("created_on")?;
    let side: Option<String> = row.get("side")?;
    let direction: String = row.get("direction")?;
    Ok(SecurityAlert {
        id: row.get("id")?,
        security_id: row.get("security_id")?,
        kind: AlertKind::parse(&kind).map_err(conversion)?,
        price: row.get::<_, Option<SqlDecimal>>("price")?.map(|p| p.0),
        currency: row.get("currency")?,
        date: optional_date(row, "date")?,
        note: row.get("note")?,
        created_on: date_from_sql(&created)?,
        side: side
            .as_deref()
            .map(AlertSide::parse)
            .transpose()
            .map_err(conversion)?,
        checked_through: optional_date(row, "checked_through")?,
        direction: AlertDirection::parse(&direction).map_err(conversion)?,
    })
}

fn row_to_crossing(row: &Row<'_>) -> rusqlite::Result<AlertCrossing> {
    let date: String = row.get("date")?;
    let direction: String = row.get("direction")?;
    Ok(AlertCrossing {
        id: row.get("id")?,
        alert_id: row.get("alert_id")?,
        date: date_from_sql(&date)?,
        direction: CrossingDirection::parse(&direction).map_err(conversion)?,
        level: row.get::<_, Option<SqlDecimal>>("level")?.map(|v| v.0),
        price: row.get::<_, Option<SqlDecimal>>("price")?.map(|v| v.0),
        currency: row.get("currency")?,
        seen: row.get::<_, i64>("seen")? != 0,
        notified: row.get::<_, i64>("notified")? != 0,
    })
}

impl Store {
    pub fn save_alert(&self, a: &SecurityAlert) -> Result<()> {
        a.validate()?;
        self.conn.execute(
            "INSERT INTO security_alerts
                 (id, security_id, kind, price, currency, date, note, created_on, side, checked_through, direction)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT (id) DO UPDATE SET
                 security_id = excluded.security_id,
                 kind = excluded.kind,
                 price = excluded.price,
                 currency = excluded.currency,
                 date = excluded.date,
                 note = excluded.note,
                 created_on = excluded.created_on,
                 side = excluded.side,
                 checked_through = excluded.checked_through,
                 direction = excluded.direction",
            params![
                a.id,
                a.security_id,
                a.kind.as_str(),
                a.price.map(dec_to_sql),
                a.currency,
                a.date.map(date_to_sql),
                a.note,
                date_to_sql(a.created_on),
                a.side.map(AlertSide::as_str),
                a.checked_through.map(date_to_sql),
                a.direction.as_str(),
            ],
        )?;
        Ok(())
    }

    pub fn get_alert(&self, id: &str) -> Result<SecurityAlert> {
        self.conn
            .query_row("SELECT * FROM security_alerts WHERE id = ?1", [id], row_to_alert)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("alert {id}")),
                other => other.into(),
            })
    }

    pub fn delete_alert(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM security_alerts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_alerts(&self) -> Result<Vec<SecurityAlert>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_alerts ORDER BY created_on, id")?;
        let rows = stmt.query_map([], row_to_alert)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn alerts_for_security(&self, security_id: &str) -> Result<Vec<SecurityAlert>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_alerts WHERE security_id = ?1 ORDER BY created_on, id")?;
        let rows = stmt.query_map([security_id], row_to_alert)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Moves the check's bookmark and appends what it crossed, in one transaction, so a crossing
    /// is never logged twice nor lost between the two writes.
    pub fn record_alert_check(
        &self,
        alert_id: &str,
        side: Option<AlertSide>,
        checked_through: Option<NaiveDate>,
        crossings: &[AlertCrossing],
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE security_alerts SET side = ?2, checked_through = ?3 WHERE id = ?1",
            params![
                alert_id,
                side.map(AlertSide::as_str),
                checked_through.map(date_to_sql)
            ],
        )?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO alert_crossings (alert_id, date, direction, level, price, currency)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for c in crossings {
                stmt.execute(params![
                    alert_id,
                    date_to_sql(c.date),
                    c.direction.as_str(),
                    c.level.map(dec_to_sql),
                    c.price.map(dec_to_sql),
                    c.currency,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The log, newest first: of one alert, or of every alert.
    pub fn alert_crossings(&self, alert_id: Option<&str>, limit: usize) -> Result<Vec<AlertCrossing>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM alert_crossings WHERE ?1 IS NULL OR alert_id = ?1
             ORDER BY date DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![alert_id, limit as i64], row_to_crossing)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn unseen_crossings(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM alert_crossings WHERE seen = 0", [], |r| {
                r.get(0)
            })?;
        Ok(n as usize)
    }

    pub fn mark_crossings_seen(&self) -> Result<usize> {
        Ok(self
            .conn
            .execute("UPDATE alert_crossings SET seen = 1 WHERE seen = 0", [])?)
    }

    /// Crossings no notification was shown for, oldest first, marked as shown in the same
    /// transaction so two callers never announce one crossing twice.
    pub fn take_unnotified_crossings(&self) -> Result<Vec<AlertCrossing>> {
        let tx = self.conn.unchecked_transaction()?;
        let taken = {
            let mut stmt =
                tx.prepare("SELECT * FROM alert_crossings WHERE notified = 0 ORDER BY date, id")?;
            let rows = stmt.query_map([], row_to_crossing)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        tx.execute("UPDATE alert_crossings SET notified = 1 WHERE notified = 0", [])?;
        tx.commit()?;
        Ok(taken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Security, SecurityKind};
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn store() -> Store {
        let store = Store::open_in_memory().unwrap();
        let mut aapl = Security::new("AAPL", "Apple", "USD", SecurityKind::Stock);
        aapl.id = "sec-aapl".into();
        store.save_security(&aapl).unwrap();
        store
    }

    #[test]
    fn a_check_moves_the_bookmark_and_logs_its_crossings() {
        let store = store();
        let alert = SecurityAlert::price("sec-aapl", dec!(200.50), "USD", d("2024-06-01")).with_note("trim");
        store.save_alert(&alert).unwrap();

        let up = AlertCrossing::new(&alert, d("2024-06-04"), CrossingDirection::Up, Some(dec!(201)));
        store
            .record_alert_check(&alert.id, Some(AlertSide::Above), Some(d("2024-06-04")), &[up])
            .unwrap();

        let read = store.get_alert(&alert.id).unwrap();
        assert_eq!(read.side, Some(AlertSide::Above));
        assert_eq!(read.checked_through, Some(d("2024-06-04")));
        let log = store.alert_crossings(Some(&alert.id), 10).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].level, Some(dec!(200.50)));
        assert_eq!(log[0].price, Some(dec!(201)));
        assert_eq!(store.unseen_crossings().unwrap(), 1);

        assert_eq!(store.take_unnotified_crossings().unwrap().len(), 1);
        assert!(
            store.take_unnotified_crossings().unwrap().is_empty(),
            "announced once"
        );
        store.mark_crossings_seen().unwrap();
        assert_eq!(store.unseen_crossings().unwrap(), 0);
    }

    #[test]
    fn deleting_the_instrument_deletes_its_alerts_and_their_log() {
        let store = store();
        let alert = SecurityAlert::date_reached("sec-aapl", d("2024-07-01"), d("2024-06-01"));
        store.save_alert(&alert).unwrap();
        let reached = AlertCrossing::new(&alert, d("2024-07-01"), CrossingDirection::Reached, None);
        store
            .record_alert_check(&alert.id, None, Some(d("2024-07-01")), &[reached])
            .unwrap();

        store.delete_security("sec-aapl").unwrap();
        assert!(store.list_alerts().unwrap().is_empty());
        assert!(store.alert_crossings(None, 10).unwrap().is_empty());
    }
}
