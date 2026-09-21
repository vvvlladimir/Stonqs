use crate::error::Result;
use rusqlite::Connection;

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
pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT NOT NULL,
             applied_at TEXT NOT NULL
         );",
    )?;

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
