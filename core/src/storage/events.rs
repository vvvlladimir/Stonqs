use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::model::{SecurityEvent, SecurityEventKind};
use rusqlite::{Row, params};

fn row_to_event(row: &Row<'_>) -> rusqlite::Result<SecurityEvent> {
    let kind: String = row.get("kind")?;
    let date: String = row.get("date")?;
    Ok(SecurityEvent {
        id: row.get("id")?,
        security_id: row.get("security_id")?,
        date: date_from_sql(&date)?,
        kind: SecurityEventKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        amount: row.get::<_, Option<SqlDecimal>>("amount")?.map(|v| v.0),
        currency: row.get("currency")?,
        ratio_from: row.get::<_, Option<SqlDecimal>>("ratio_from")?.map(|v| v.0),
        ratio_to: row.get::<_, Option<SqlDecimal>>("ratio_to")?.map(|v| v.0),
        note: row.get("note")?,
        source: row.get("source")?,
    })
}

impl Store {
    /// Writes one event by id — the user's note, or an edit of a reported event.
    pub fn save_security_event(&self, e: &SecurityEvent) -> Result<()> {
        e.validate()?;
        self.conn.execute(
            "INSERT INTO security_events
                 (id, security_id, date, kind, amount, currency, ratio_from, ratio_to, note, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (id) DO UPDATE SET
                 security_id = excluded.security_id,
                 date = excluded.date,
                 kind = excluded.kind,
                 amount = excluded.amount,
                 currency = excluded.currency,
                 ratio_from = excluded.ratio_from,
                 ratio_to = excluded.ratio_to,
                 note = excluded.note,
                 source = excluded.source",
            params![
                e.id,
                e.security_id,
                date_to_sql(e.date),
                e.kind.as_str(),
                e.amount.map(dec_to_sql),
                e.currency,
                e.ratio_from.map(dec_to_sql),
                e.ratio_to.map(dec_to_sql),
                e.note,
                e.source,
            ],
        )?;
        Ok(())
    }

    /// Saves what a provider reported. One dividend or split per instrument and day: a second
    /// fetch of the same range updates the row and keeps its id.
    pub fn save_provider_events(&self, events: &[SecurityEvent]) -> Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO security_events
                     (id, security_id, date, kind, amount, currency, ratio_from, ratio_to, note, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (security_id, kind, date) WHERE source IS NOT NULL DO UPDATE SET
                     amount = excluded.amount,
                     currency = excluded.currency,
                     ratio_from = excluded.ratio_from,
                     ratio_to = excluded.ratio_to,
                     source = excluded.source",
            )?;
            for e in events {
                e.validate()?;
                if e.source.is_none() {
                    return Err(Error::Invalid("a provider event needs its source".into()));
                }
                stmt.execute(params![
                    e.id,
                    e.security_id,
                    date_to_sql(e.date),
                    e.kind.as_str(),
                    e.amount.map(dec_to_sql),
                    e.currency,
                    e.ratio_from.map(dec_to_sql),
                    e.ratio_to.map(dec_to_sql),
                    e.note,
                    e.source,
                ])?;
            }
        }
        tx.commit()?;
        Ok(events.len())
    }

    pub fn get_security_event(&self, id: &str) -> Result<SecurityEvent> {
        self.conn
            .query_row("SELECT * FROM security_events WHERE id = ?1", [id], row_to_event)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("event {id}")),
                other => other.into(),
            })
    }

    pub fn delete_security_event(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM security_events WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Every event, newest first.
    pub fn list_security_events(&self) -> Result<Vec<SecurityEvent>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_events ORDER BY date DESC, id")?;
        let rows = stmt.query_map([], row_to_event)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn security_events_for(&self, security_id: &str) -> Result<Vec<SecurityEvent>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_events WHERE security_id = ?1 ORDER BY date DESC, id")?;
        let rows = stmt.query_map([security_id], row_to_event)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Date range already asked of the provider for events.
    pub fn event_coverage(&self, security_id: &str) -> Result<Option<DateRange>> {
        let mut stmt = self
            .conn
            .prepare("SELECT from_date, to_date FROM event_coverage WHERE security_id = ?1")?;
        let mut rows = stmt.query_map([security_id], |r| {
            let f: String = r.get(0)?;
            let t: String = r.get(1)?;
            Ok(DateRange::new(date_from_sql(&f)?, date_from_sql(&t)?))
        })?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn extend_event_coverage(&self, security_id: &str, range: DateRange) -> Result<()> {
        let merged = match self.event_coverage(security_id)? {
            Some(old) => DateRange::new(old.from.min(range.from), old.to.max(range.to)),
            None => range,
        };
        self.conn.execute(
            "INSERT INTO event_coverage (security_id, from_date, to_date) VALUES (?1, ?2, ?3)
             ON CONFLICT (security_id) DO UPDATE SET
                 from_date = excluded.from_date,
                 to_date = excluded.to_date",
            params![security_id, date_to_sql(merged.from), date_to_sql(merged.to)],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::Quote;
    use crate::model::{Security, SecurityKind};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn store() -> Store {
        let store = Store::open_in_memory().unwrap();
        let mut nvda = Security::new("NVDA", "NVIDIA", "USD", SecurityKind::Stock);
        nvda.id = "sec-nvda".into();
        store.save_security(&nvda).unwrap();
        store
    }

    #[test]
    fn a_second_fetch_updates_the_reported_event_instead_of_adding_one() {
        let store = store();
        let first = SecurityEvent::dividend("sec-nvda", d("2024-06-11"), dec!(0.01), "USD", "yahoo");
        store.save_provider_events(std::slice::from_ref(&first)).unwrap();
        let again = SecurityEvent::dividend("sec-nvda", d("2024-06-11"), dec!(0.010), "USD", "yahoo");
        store.save_provider_events(&[again]).unwrap();

        let events = store.security_events_for("sec-nvda").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].id, first.id,
            "the row keeps the id it was first written with"
        );
    }

    #[test]
    fn a_note_on_the_same_day_is_not_a_conflict() {
        let store = store();
        let split = SecurityEvent::split("sec-nvda", d("2024-06-10"), dec!(1), dec!(10), "yahoo");
        store.save_provider_events(&[split]).unwrap();
        store
            .save_security_event(&SecurityEvent::note("sec-nvda", d("2024-06-10"), "10-for-1"))
            .unwrap();
        assert_eq!(store.security_events_for("sec-nvda").unwrap().len(), 2);
    }

    /// A different listing reports in a different currency, so what the old one said goes with
    /// its quotes; the user's notes are about the instrument and stay.
    #[test]
    fn switching_the_listing_drops_reported_events_and_keeps_notes() {
        let store = store();
        store
            .save_quotes(&[Quote {
                security_id: "sec-nvda".into(),
                date: d("2024-06-10"),
                close: dec!(121.79),
                currency: "USD".into(),
                source: "yahoo".into(),
            }])
            .unwrap();
        let dividend = SecurityEvent::dividend("sec-nvda", d("2024-06-11"), dec!(0.01), "USD", "yahoo");
        store.save_provider_events(&[dividend]).unwrap();
        store
            .save_security_event(&SecurityEvent::note("sec-nvda", d("2024-06-12"), "earnings"))
            .unwrap();
        let range = DateRange::new(d("2024-01-01"), d("2024-06-12"));
        store.extend_event_coverage("sec-nvda", range).unwrap();

        store.delete_quotes("sec-nvda").unwrap();

        let events = store.security_events_for("sec-nvda").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, SecurityEventKind::Note);
        assert_eq!(store.event_coverage("sec-nvda").unwrap(), None);
    }
}
