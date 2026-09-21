//! SQLite storage backed by `rusqlite`; see ADR-001 for the design rationale.

mod account_groups;
mod accounts;
mod ai_chats;
mod ai_usage;
mod alerts;
mod attributes;
mod corporate_actions;
mod events;
mod fx_rates;
mod listings;
mod migrate;
mod plans;
mod portfolios;
mod price_index;
mod quotes;
mod securities;
mod targets;
mod taxonomies;
mod transactions;
mod watchlists;

pub use attributes::AttributeValues;
pub use migrate::MIGRATIONS;
pub use quotes::QuoteStats;

use crate::error::Result;
use chrono::NaiveDate;
use rusqlite::Connection;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

/// SQLCipher's spelling of a raw key (`x'…'`); an empty key means an unencrypted database.
fn raw_key(key: Option<&[u8; 32]>) -> String {
    match key {
        Some(bytes) => {
            let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
            format!("x'{hex}'")
        }
        None => String::new(),
    }
}

/// Database connection with migrations applied.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens or creates a database file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Opens or creates a database file encrypted with SQLCipher under a raw 256-bit key — raw, so
    /// SQLCipher runs no key derivation of its own; the caller's key is already a random one. A
    /// wrong key fails here, on the first read, never later.
    pub fn open_encrypted(path: impl AsRef<Path>, key: &[u8; 32]) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(&format!("PRAGMA key = \"{}\";", raw_key(Some(key))))?;
        Self::init(conn)
    }

    /// Writes a complete copy of this database to `dest` — encrypted under `key`, or plain when
    /// there is none. This is how a file changes between plain and encrypted: SQLCipher cannot
    /// encrypt or decrypt a database in place. `dest` must not exist yet.
    pub fn export_to(&self, dest: impl AsRef<Path>, key: Option<&[u8; 32]>) -> Result<()> {
        let dest = dest.as_ref().to_string_lossy().into_owned();
        self.conn
            .execute("ATTACH DATABASE ?1 AS export KEY ?2", (&dest, raw_key(key)))?;
        let exported = self
            .conn
            .query_row("SELECT sqlcipher_export('export')", [], |_| Ok(()));
        // Detached whatever happened, so a failed export does not leave the file held open.
        self.conn.execute_batch("DETACH DATABASE export")?;
        exported?;
        Ok(())
    }

    /// Folds the write-ahead log into the file and empties it, so the database is one file.
    pub fn checkpoint(&self) -> Result<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    /// Opens an in-memory database for tests and CLI demos.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        // Keep referential actions enabled and let concurrent readers wait for writers.
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;",
        )?;
        let store = Store { conn };
        migrate::run(&store.conn)?;
        Ok(store)
    }

    /// Returns every currency used by accounts, securities, or transactions.
    pub fn distinct_currencies(&self) -> Result<std::collections::BTreeSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT currency FROM accounts
             UNION SELECT currency FROM securities
             UNION SELECT currency FROM transactions",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = std::collections::BTreeSet::new();
        for row in rows {
            out.insert(row?);
        }
        Ok(out)
    }

    /// Exposes the connection to CLI tooling and custom reports.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

// Decimal and dates are stored as TEXT; conversions stay centralized here.

/// Reads a `Decimal` stored in a SQLite TEXT column.
pub(crate) struct SqlDecimal(pub Decimal);

impl FromSql for SqlDecimal {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        Decimal::from_str(s)
            .map(SqlDecimal)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

pub(crate) fn dec_to_sql(v: Decimal) -> String {
    v.to_string()
}

pub(crate) fn date_to_sql(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn date_from_sql(s: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vs-store-{name}-{}.db", uuid::Uuid::new_v4()))
    }

    /// Plain to encrypted and back: the data survives both ways, the encrypted file opens only
    /// with its key, and its bytes do not carry the data in the clear.
    #[test]
    fn a_database_moves_between_plain_and_encrypted() {
        let key = [7u8; 32];
        let (plain, sealed, back) = (temp("plain"), temp("sealed"), temp("back"));

        let store = Store::open(&plain).unwrap();
        store
            .save_portfolio(&crate::model::Portfolio::new("Findable name", "EUR"))
            .unwrap();
        store.export_to(&sealed, Some(&key)).unwrap();
        drop(store);

        let raw = std::fs::read(&sealed).unwrap();
        assert!(!raw.windows(13).any(|w| w == b"Findable name"));
        assert!(Store::open(&sealed).is_err(), "no key, no database");
        assert!(
            Store::open_encrypted(&sealed, &[8u8; 32]).is_err(),
            "a wrong key fails at once"
        );

        let opened = Store::open_encrypted(&sealed, &key).unwrap();
        assert_eq!(opened.list_portfolios().unwrap()[0].name, "Findable name");
        opened.export_to(&back, None).unwrap();
        drop(opened);
        assert_eq!(
            Store::open(&back).unwrap().list_portfolios().unwrap()[0].name,
            "Findable name"
        );

        for path in [plain, sealed, back] {
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
            }
        }
    }

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let store = Store::open_in_memory().unwrap();
        // Re-running migrations must be idempotent.
        migrate::run(store.conn()).unwrap();
        let n: i64 = store
            .conn()
            .query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n as usize, MIGRATIONS.len());
    }

    /// A database at an older schema version catches up safely.
    #[test]
    fn a_database_stopped_at_an_older_version_catches_up() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                 version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL
             );",
        )
        .unwrap();
        // Simulate a database created by the previous application version.
        for (version, name, sql) in &MIGRATIONS[..MIGRATIONS.len() - 1] {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, '')",
                rusqlite::params![version, name],
            )
            .unwrap();
        }

        migrate::run(&conn).unwrap();

        // The latest columns exist and a second run remains idempotent.
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('listings')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        for expected in ["name", "last_close", "currency", "has_history", "symbol"] {
            assert!(
                columns.contains(&expected.to_string()),
                "missing column {expected}"
            );
        }
        migrate::run(&conn).unwrap();
    }
}
