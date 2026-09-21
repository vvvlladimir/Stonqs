use super::{SqlDecimal, Store, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{Security, SecurityKind};
use rusqlite::{Row, params};

fn row_to_security(row: &Row<'_>) -> rusqlite::Result<Security> {
    let kind: String = row.get("kind")?;
    Ok(Security {
        id: row.get("id")?,
        symbol: row.get("symbol")?,
        isin: row.get("isin")?,
        name: row.get("name")?,
        currency: row.get("currency")?,
        kind: SecurityKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        data_source: row.get("data_source")?,
        data_symbol: row.get("data_symbol")?,
        quantity_step: row.get::<_, Option<SqlDecimal>>("quantity_step")?.map(|d| d.0),
        mic: row.get("mic")?,
        wkn: row.get("wkn")?,
        note: row.get("note")?,
    })
}

impl Store {
    pub fn save_security(&self, s: &Security) -> Result<()> {
        self.conn.execute(
            "INSERT INTO securities
                 (id, symbol, isin, name, currency, kind, data_source, data_symbol, quantity_step,
                  mic, wkn, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (id) DO UPDATE SET
                 symbol = excluded.symbol,
                 isin = excluded.isin,
                 name = excluded.name,
                 currency = excluded.currency,
                 kind = excluded.kind,
                 data_source = excluded.data_source,
                 data_symbol = excluded.data_symbol,
                 quantity_step = excluded.quantity_step,
                 mic = excluded.mic,
                 wkn = excluded.wkn,
                 note = excluded.note",
            params![
                s.id,
                s.symbol,
                s.isin,
                s.name,
                s.currency,
                s.kind.as_str(),
                s.data_source,
                s.data_symbol,
                s.quantity_step.map(dec_to_sql),
                s.mic,
                s.wkn,
                s.note
            ],
        )?;
        Ok(())
    }

    pub fn get_security(&self, id: &str) -> Result<Security> {
        self.conn
            .query_row("SELECT * FROM securities WHERE id = ?1", [id], row_to_security)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("security {id}")),
                other => other.into(),
            })
    }

    /// Finds a security by ticker for broker-report imports.
    pub fn find_security_by_symbol(&self, symbol: &str) -> Result<Option<Security>> {
        let mut stmt = self.conn.prepare("SELECT * FROM securities WHERE symbol = ?1")?;
        let mut rows = stmt.query_map([symbol], row_to_security)?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn list_securities(&self) -> Result<Vec<Security>> {
        let mut stmt = self.conn.prepare("SELECT * FROM securities ORDER BY symbol")?;
        let rows = stmt.query_map([], row_to_security)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Deletes a security only when no transaction references it.
    pub fn delete_security(&self, id: &str) -> Result<()> {
        let used: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM transactions WHERE security_id = ?1",
            [id],
            |row| row.get(0),
        )?;
        if used > 0 {
            return Err(Error::Invalid(format!(
                "the instrument is used by {used} transactions; delete them first"
            )));
        }
        self.conn.execute("DELETE FROM securities WHERE id = ?1", [id])?;
        self.forget_taxonomy_subject(id)?;
        Ok(())
    }
}

impl Store {
    /// The instrument's symbol at `source`: its own `provider_symbol` when that is its source,
    /// otherwise what `security_symbols` recorded. `None` means the source does not know it.
    pub fn symbol_at(&self, security: &Security, source: &str) -> Result<Option<String>> {
        if security.data_source.as_deref() == Some(source) {
            return Ok(Some(security.provider_symbol().to_string()));
        }
        let mut stmt = self
            .conn
            .prepare("SELECT symbol FROM security_symbols WHERE security_id = ?1 AND source = ?2")?;
        let mut rows = stmt.query_map(params![security.id, source], |r| r.get::<_, String>(0))?;
        rows.next().transpose().map_err(Into::into)
    }

    /// Every alternative symbol of an instrument, by source.
    pub fn security_symbols(&self, security_id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source, symbol FROM security_symbols WHERE security_id = ?1 ORDER BY source")?;
        let rows = stmt.query_map([security_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Every instrument's alternative symbols, `security_id -> source -> symbol`, in one read.
    pub fn all_security_symbols(
        &self,
    ) -> Result<std::collections::HashMap<String, std::collections::BTreeMap<String, String>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT security_id, source, symbol FROM security_symbols")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut out: std::collections::HashMap<String, std::collections::BTreeMap<String, String>> =
            Default::default();
        for row in rows {
            let (id, source, symbol) = row?;
            out.entry(id).or_default().insert(source, symbol);
        }
        Ok(out)
    }

    /// Records (or, with an empty symbol, forgets) the instrument's symbol at another source.
    pub fn set_security_symbol(&self, security_id: &str, source: &str, symbol: &str) -> Result<()> {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            self.conn.execute(
                "DELETE FROM security_symbols WHERE security_id = ?1 AND source = ?2",
                params![security_id, source],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO security_symbols (security_id, source, symbol) VALUES (?1, ?2, ?3)
                 ON CONFLICT (security_id, source) DO UPDATE SET symbol = excluded.symbol",
                params![security_id, source, symbol],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod symbol_tests {
    use super::*;

    #[test]
    fn the_own_source_reads_the_security_and_others_read_the_table() {
        let store = Store::open_in_memory().unwrap();
        let btc =
            Security::new("BTC", "Bitcoin", "EUR", SecurityKind::Crypto).with_source("yahoo", "BTC-EUR");
        store.save_security(&btc).unwrap();
        store.set_security_symbol(&btc.id, "kraken", "XXBTZEUR").unwrap();

        assert_eq!(
            store.symbol_at(&btc, "yahoo").unwrap().as_deref(),
            Some("BTC-EUR")
        );
        assert_eq!(
            store.symbol_at(&btc, "kraken").unwrap().as_deref(),
            Some("XXBTZEUR")
        );
        assert_eq!(store.symbol_at(&btc, "stooq").unwrap(), None);

        store.set_security_symbol(&btc.id, "kraken", " ").unwrap();
        assert!(store.security_symbols(&btc.id).unwrap().is_empty());
    }
}
