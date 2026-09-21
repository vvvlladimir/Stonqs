use crate::error::{Error, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// How many upgrade copies are kept beside the database. A backup is a whole database file, so
/// every upgrade ever applied would eventually cost more than the portfolio it protects.
const KEEP_BACKUPS: usize = 3;

/// Migrations in application order; applied files must never be edited.
pub const MIGRATIONS: &[(i64, &str, &str)] = &[
    (1, "init", include_str!("../../migrations/0001_init.sql")),
    (
        2,
        "transfers_splits_costbasis",
        include_str!("../../migrations/0002_transfers_splits_costbasis.sql"),
    ),
    (
        3,
        "taxonomies",
        include_str!("../../migrations/0003_taxonomies.sql"),
    ),
    (4, "targets", include_str!("../../migrations/0004_targets.sql")),
    (5, "listings", include_str!("../../migrations/0005_listings.sql")),
    (
        6,
        "listing_probe",
        include_str!("../../migrations/0006_listing_probe.sql"),
    ),
    (
        7,
        "deposit_accounts",
        include_str!("../../migrations/0007_deposit_accounts.sql"),
    ),
    (
        8,
        "security_mic",
        include_str!("../../migrations/0008_security_mic.sql"),
    ),
    (
        9,
        "taxonomy_node_color",
        include_str!("../../migrations/0009_taxonomy_node_color.sql"),
    ),
    (
        10,
        "default_taxonomies",
        include_str!("../../migrations/0010_default_taxonomies.sql"),
    ),
    (
        11,
        "cash_and_exclusions",
        include_str!("../../migrations/0011_cash_and_exclusions.sql"),
    ),
    (
        12,
        "security_attributes",
        include_str!("../../migrations/0012_security_attributes.sql"),
    ),
    (
        13,
        "investment_plans",
        include_str!("../../migrations/0013_investment_plans.sql"),
    ),
    (
        14,
        "security_alerts",
        include_str!("../../migrations/0014_security_alerts.sql"),
    ),
    (
        15,
        "alert_crossings",
        include_str!("../../migrations/0015_alert_crossings.sql"),
    ),
    (
        16,
        "alert_direction",
        include_str!("../../migrations/0016_alert_direction.sql"),
    ),
    (
        17,
        "watchlists",
        include_str!("../../migrations/0017_watchlists.sql"),
    ),
    (
        18,
        "ai_assistant",
        include_str!("../../migrations/0018_ai_assistant.sql"),
    ),
    (
        19,
        "ai_grants",
        include_str!("../../migrations/0019_ai_grants.sql"),
    ),
    (
        20,
        "ai_chat_tool_mode",
        include_str!("../../migrations/0020_ai_chat_tool_mode.sql"),
    ),
    (
        21,
        "ai_chat_effort",
        include_str!("../../migrations/0021_ai_chat_effort.sql"),
    ),
    (22, "ai_usage", include_str!("../../migrations/0022_ai_usage.sql")),
    (
        23,
        "market_sources",
        include_str!("../../migrations/0023_market_sources.sql"),
    ),
    (
        24,
        "source_usage",
        include_str!("../../migrations/0024_source_usage.sql"),
    ),
    (
        25,
        "price_index",
        include_str!("../../migrations/0025_price_index.sql"),
    ),
];

/// Applies all pending migrations.
///
/// `path` is the database's own file when it has one, and is what makes the pre-upgrade copy
/// possible; an in-memory database passes `None` and is never backed up.
pub fn run(conn: &Connection, path: Option<&Path>) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT NOT NULL,
             applied_at TEXT NOT NULL
         );",
    )?;

    // A database that has never been migrated holds nothing to lose, so only an *upgrade* is
    // copied: a fresh file would otherwise leave an empty backup beside every new profile.
    let from: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;
    let pending = MIGRATIONS.iter().any(|(v, _, _)| *v > from);
    if from > 0
        && pending
        && let Some(path) = path
    {
        back_up(conn, path, from)?;
    }

    for (version, name, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |r| r.get(0),
        )?;
        if already {
            continue;
        }
        // Each migration is atomic so a partial SQL failure leaves no changes.
        conn.execute_batch("BEGIN;")?;
        let applied = conn.execute_batch(sql).and_then(|_| {
            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, datetime('now'))",
                rusqlite::params![version, name],
            )
            .map(|_| ())
        });
        match applied {
            Ok(()) => conn.execute_batch("COMMIT;")?,
            Err(e) => {
                conn.execute_batch("ROLLBACK;")?;
                return Err(e.into());
            }
        }
    }
    Ok(())
}

/// Copies the database beside itself as `<name>.bak-v<from>` before an upgrade touches it.
///
/// Each migration is atomic on its own, but a *sequence* of them is not undoable: a version that
/// drops a column cannot give it back. An encrypted database copies as the encrypted bytes it
/// already is, so the backup is no weaker than the original.
fn back_up(conn: &Connection, path: &Path, from: i64) -> Result<()> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(Error::Backup(format!("{} is not a file", path.display())));
    };
    // Committed pages can still be sitting in the -wal file; a byte copy taken without this
    // would silently be missing the most recent transactions.
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;

    let target = path.with_file_name(format!("{name}.bak-v{from}"));
    std::fs::copy(path, &target)
        .map_err(|e| Error::Backup(format!("{} -> {}: {e}", path.display(), target.display())))?;
    prune_backups(path, name);
    Ok(())
}

/// Deletes all but the [`KEEP_BACKUPS`] newest copies. A failure here is not the caller's
/// problem: the backup that matters has already been written, and a full disk is a better
/// complaint from the next upgrade than from this one.
fn prune_backups(path: &Path, name: &str) {
    let Some(dir) = path.parent() else { return };
    let prefix = format!("{name}.bak-v");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut found: Vec<(i64, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name();
            let version: i64 = file.to_str()?.strip_prefix(&prefix)?.parse().ok()?;
            Some((version, e.path()))
        })
        .collect();
    found.sort_by_key(|(version, _)| *version);
    let drop_count = found.len().saturating_sub(KEEP_BACKUPS);
    for (_, old) in found.into_iter().take(drop_count) {
        let _ = std::fs::remove_file(old);
    }
}
