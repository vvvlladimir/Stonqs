//! The documentation corpora, compiled in and fetched one file at a time rather than put in the
//! prompt (ADR-0038).

/// Domain concepts for the model; slug = file name = the `topic` argument.
const REFERENCE: &[(&str, &str)] = &[
    ("alerts", include_str!("../../../../docs/ai-reference/alerts.md")),
    (
        "as-of-date",
        include_str!("../../../../docs/ai-reference/as-of-date.md"),
    ),
    (
        "allocation-and-taxonomy",
        include_str!("../../../../docs/ai-reference/allocation-and-taxonomy.md"),
    ),
    (
        "benchmark",
        include_str!("../../../../docs/ai-reference/benchmark.md"),
    ),
    (
        "calculation-sheet",
        include_str!("../../../../docs/ai-reference/calculation-sheet.md"),
    ),
    (
        "corporate-actions",
        include_str!("../../../../docs/ai-reference/corporate-actions.md"),
    ),
    (
        "cost-basis-and-lots",
        include_str!("../../../../docs/ai-reference/cost-basis-and-lots.md"),
    ),
    (
        "currency-and-fx",
        include_str!("../../../../docs/ai-reference/currency-and-fx.md"),
    ),
    (
        "data-sources",
        include_str!("../../../../docs/ai-reference/data-sources.md"),
    ),
    (
        "data-scope",
        include_str!("../../../../docs/ai-reference/data-scope.md"),
    ),
    (
        "goals-and-limits",
        include_str!("../../../../docs/ai-reference/goals-and-limits.md"),
    ),
    (
        "importing-data",
        include_str!("../../../../docs/ai-reference/importing-data.md"),
    ),
    (
        "net-worth",
        include_str!("../../../../docs/ai-reference/net-worth.md"),
    ),
    (
        "income-and-costs",
        include_str!("../../../../docs/ai-reference/income-and-costs.md"),
    ),
    (
        "inflation-and-real-return",
        include_str!("../../../../docs/ai-reference/inflation-and-real-return.md"),
    ),
    (
        "instruments-and-listings",
        include_str!("../../../../docs/ai-reference/instruments-and-listings.md"),
    ),
    (
        "investment-plans",
        include_str!("../../../../docs/ai-reference/investment-plans.md"),
    ),
    (
        "periods",
        include_str!("../../../../docs/ai-reference/periods.md"),
    ),
    (
        "rebalancing",
        include_str!("../../../../docs/ai-reference/rebalancing.md"),
    ),
    (
        "returns-twr-xirr",
        include_str!("../../../../docs/ai-reference/returns-twr-xirr.md"),
    ),
    (
        "risk-metrics",
        include_str!("../../../../docs/ai-reference/risk-metrics.md"),
    ),
    (
        "valuation-and-prices",
        include_str!("../../../../docs/ai-reference/valuation-and-prices.md"),
    ),
    (
        "watchlists",
        include_str!("../../../../docs/ai-reference/watchlists.md"),
    ),
];

/// One guide per screen id; a missing one answers "no guide" rather than inviting invention.
const GUIDES: &[(&str, &str)] = &[
    ("plugin", include_str!("../../../../docs/user-guide/plugin.md")),
    (
        "dashboard",
        include_str!("../../../../docs/user-guide/dashboard.md"),
    ),
    (
        "positions",
        include_str!("../../../../docs/user-guide/positions.md"),
    ),
    (
        "transactions",
        include_str!("../../../../docs/user-guide/transactions.md"),
    ),
    (
        "accounts",
        include_str!("../../../../docs/user-guide/accounts.md"),
    ),
    ("plans", include_str!("../../../../docs/user-guide/plans.md")),
    (
        "networth",
        include_str!("../../../../docs/user-guide/networth.md"),
    ),
    ("alerts", include_str!("../../../../docs/user-guide/alerts.md")),
    (
        "securities",
        include_str!("../../../../docs/user-guide/securities.md"),
    ),
    (
        "watchlist",
        include_str!("../../../../docs/user-guide/watchlist.md"),
    ),
    (
        "performance",
        include_str!("../../../../docs/user-guide/performance.md"),
    ),
    ("trades", include_str!("../../../../docs/user-guide/trades.md")),
    ("risk", include_str!("../../../../docs/user-guide/risk.md")),
    (
        "allocation",
        include_str!("../../../../docs/user-guide/allocation.md"),
    ),
    ("income", include_str!("../../../../docs/user-guide/income.md")),
    (
        "rebalance",
        include_str!("../../../../docs/user-guide/rebalance.md"),
    ),
    ("import", include_str!("../../../../docs/user-guide/import.md")),
    ("reports", include_str!("../../../../docs/user-guide/reports.md")),
    (
        "settings",
        include_str!("../../../../docs/user-guide/settings.md"),
    ),
];

/// The frontend's `ScreenId` union, mirrored so the host can name a screen in a tool schema.
/// Kept honest by a test against `app/src/lib/nav.tsx` rather than by discipline.
pub const SCREEN_IDS: &[&str] = &[
    "dashboard",
    "positions",
    "transactions",
    "accounts",
    "plans",
    "networth",
    "alerts",
    "securities",
    "watchlist",
    "performance",
    "trades",
    "risk",
    "allocation",
    "rebalance",
    "income",
    "import",
    "reports",
    "settings",
    "plugin",
];

pub fn topics() -> Vec<&'static str> {
    REFERENCE.iter().map(|(topic, _)| *topic).collect()
}

pub fn reference(topic: &str) -> Option<&'static str> {
    REFERENCE
        .iter()
        .find(|(name, _)| *name == topic)
        .map(|(_, text)| *text)
}

pub fn guide(screen: &str) -> Option<&'static str> {
    GUIDES
        .iter()
        .find(|(name, _)| *name == screen)
        .map(|(_, text)| *text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Both corpora carry one of these, and it is documentation about the directory rather than
    /// an entry in it.
    const NOT_AN_ENTRY: &str = "README.md";

    fn slugs_in(dir: &str) -> BTreeSet<String> {
        std::fs::read_dir(Path::new("../..").join(dir))
            .unwrap_or_else(|e| panic!("{dir}: {e}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".md") && name != NOT_AN_ENTRY)
            .map(|name| name.trim_end_matches(".md").to_string())
            .collect()
    }

    /// The check runs in this direction on purpose. "Every screen must have a guide" would block
    /// partial coverage, which is the plan; "every guide must name a screen that still exists"
    /// catches the thing that rots silently — a screen renamed, its guide left behind under a
    /// dead name, and the model reading it for a screen nobody can open.
    #[test]
    fn every_guide_is_named_after_a_screen_that_exists() {
        for slug in slugs_in("docs/user-guide") {
            assert!(
                SCREEN_IDS.contains(&slug.as_str()),
                "docs/user-guide/{slug}.md names no screen; rename it or delete it"
            );
        }
    }

    /// A file added to either corpus but not to the table is invisible to the model, and the
    /// absence looks exactly like "we never wrote that one".
    #[test]
    fn every_file_in_both_corpora_is_reachable() {
        for slug in slugs_in("docs/ai-reference") {
            assert!(
                reference(&slug).is_some(),
                "docs/ai-reference/{slug}.md is not in REFERENCE and no tool can reach it"
            );
        }
        for slug in slugs_in("docs/user-guide") {
            assert!(
                guide(&slug).is_some(),
                "docs/user-guide/{slug}.md is not in GUIDES and no tool can reach it"
            );
        }
    }

    /// The screen ids are the frontend's, so they are read back from the frontend. The same
    /// trick `wire_format.rs` plays: a copy is fine as long as drifting from the original fails.
    #[test]
    fn the_screen_ids_match_the_frontends_union() {
        let source = std::fs::read_to_string("../src/lib/nav.tsx").expect("nav.tsx");
        let (_, after) = source.split_once("export type ScreenId =").expect("the union");
        let (union, _) = after.split_once(';').expect("the union ends");

        let declared: Vec<&str> = union
            .split('"')
            // The ids sit between the quotes: every odd slice of the split.
            .skip(1)
            .step_by(2)
            .collect();

        assert_eq!(declared, SCREEN_IDS, "SCREEN_IDS drifted from nav.tsx");
    }
}
