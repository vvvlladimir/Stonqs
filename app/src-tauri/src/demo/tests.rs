use super::*;
use sq_core::market::DateRange;

/// The demo as the app sees it: an in-memory database, the same seed, and then the readings
/// every screen makes. A generated history that cannot be valued is worse than none.
fn seeded() -> (Store, Portfolio, NaiveDate) {
    let store = Store::open_in_memory().expect("an in-memory store");
    let today = NaiveDate::from_ymd_opt(2026, 6, 15).expect("a valid date");
    let mut portfolio = Portfolio::new("Demo", "EUR");
    seed(&store, &mut portfolio, today).expect("the demo seeds");
    (store, portfolio, today)
}

#[test]
fn every_figure_on_the_dashboard_resolves() {
    let (store, portfolio, today) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).expect("analytics");

    let valuation = analytics.valuation_at(today).expect("a valuation");
    assert!(valuation.total_value_base > dec!(20000), "{valuation:?}");
    assert!(valuation.securities_value_base > Decimal::ZERO);
    assert!(
        valuation.cash_base > Decimal::ZERO,
        "cash ran negative: {valuation:?}"
    );
    assert!(valuation.dividends_base > Decimal::ZERO);
    assert!(valuation.realized_pnl_base != Decimal::ZERO, "no closed trade");

    let year = DateRange::new(today - chrono::Duration::days(365), today);
    analytics.risk(year, 0.02).expect("risk over a year");
    analytics.twr(year.from, year.to).expect("a TWR");
    analytics.xirr(today).expect("an IRR");
    analytics
        .benchmark_growth(&test_security(&store, "CSPX"), &[year.from, year.to])
        .expect("a benchmark series");
}

#[test]
fn the_asset_class_tree_classifies_everything() {
    let (store, portfolio, today) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).expect("analytics");
    let allocation = analytics
        .allocation_by_taxonomy("tax-asset-class", today)
        .expect("an allocation");
    let unclassified = allocation
        .buckets
        .iter()
        .find(|b| b.key == "UNCLASSIFIED")
        .map(|b| b.weight)
        .unwrap_or_default();
    assert_eq!(unclassified, Decimal::ZERO, "{allocation:?}");
}

#[test]
fn the_extras_are_readable() {
    let (store, portfolio, _) = seeded();
    assert_eq!(store.list_goals(&portfolio.id).expect("goals").len(), 2);
    assert_eq!(store.list_limits().expect("limits").len(), 2);
    assert_eq!(store.list_plans(&portfolio.id).expect("plans").len(), 2);
    assert_eq!(store.list_watchlists().expect("watchlists").len(), 1);
    assert_eq!(store.list_alerts().expect("alerts").len(), 3);
    assert_eq!(
        store.targets_for_portfolio(&portfolio.id).expect("targets").len(),
        1
    );
    assert_eq!(store.list_attribute_defs().expect("attributes").len(), 2);
    assert_eq!(store.list_assets(&portfolio.id).expect("assets").len(), 3);
}

/// The demo's net worth adds the flat and the car and subtracts the mortgage, and the portfolio's
/// own total is untouched by all three (ADR-0092).
#[test]
fn net_worth_is_the_portfolio_plus_what_is_owned_minus_what_is_owed() {
    let (store, portfolio, today) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).expect("analytics");

    let valuation = analytics.valuation_at(today).expect("a valuation");
    let reading = analytics.net_worth(today).expect("net worth");

    assert_eq!(reading.investments_base, valuation.total_value_base);
    assert_eq!(
        reading.net_base,
        valuation.total_value_base + reading.owned_base - reading.owed_base
    );
    assert!(reading.owed_base > Decimal::ZERO, "{reading:?}");
    assert!(reading.not_valued_yet.is_empty(), "{reading:?}");
    // Every figure is months old at the latest, so each row can name its own day.
    assert!(
        reading.holdings.iter().all(|h| h.valued_on < today),
        "{reading:?}"
    );
    assert!(
        !analytics
            .net_worth_series(today - chrono::Duration::days(365), today)
            .expect("a line")
            .points
            .is_empty()
    );
}

fn test_security(store: &Store, symbol: &str) -> String {
    store
        .list_securities()
        .expect("securities")
        .into_iter()
        .find(|s| s.symbol == symbol)
        .expect("the demo holds that symbol")
        .id
}
