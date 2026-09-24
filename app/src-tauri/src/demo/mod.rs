//! The demo portfolio offered to a new profile.
//!
//! It ships in every build, not just a debug one: somebody who has just downloaded this is not
//! going to hand their broker statement to an application they have never seen, and a portfolio
//! they can click through is the only honest way to show what it does. Everything is dated
//! backwards from today, so every period on the strip covers something.

mod extras;
mod ledger;
mod market;

use chrono::{Datelike, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::prelude::*;
use std::collections::BTreeMap;

/// How far back the history reaches. Three years covers the 1Y and 3Y periods and gives the
/// monthly return heatmap enough rows to read as one.
const YEARS: u32 = 3;

/// The demo's accounts, named rather than indexed: a ledger line saying `usd_depot` is readable,
/// one saying `accounts[3]` is not.
pub struct Accounts {
    pub euro_cash: Account,
    pub euro_depot: Account,
    pub usd_cash: Account,
    pub usd_depot: Account,
    pub savings: Account,
}

/// The instruments, same reasoning. `sp500` and `nvidia` are never held — one is what the
/// portfolio is compared against, the other is what a watchlist is for.
pub struct Instruments {
    pub apple: Security,
    pub microsoft: Security,
    pub netflix: Security,
    pub allworld: Security,
    pub core_world: Security,
    pub bonds: Security,
    pub bitcoin: Security,
    pub sp500: Security,
    pub nvidia: Security,
}

impl Instruments {
    fn all(&self) -> [&Security; 9] {
        [
            &self.apple,
            &self.microsoft,
            &self.netflix,
            &self.allworld,
            &self.core_world,
            &self.bonds,
            &self.bitcoin,
            &self.sp500,
            &self.nvidia,
        ]
    }
}

/// Everything the ledger and the extras read: the trading calendar, the generated closes, and
/// who holds them.
pub struct World {
    pub today: NaiveDate,
    pub days: Vec<NaiveDate>,
    /// One USD in EUR, per trading day.
    pub fx: Vec<Decimal>,
    /// Closes per security id, one entry per day in `days`.
    pub prices: BTreeMap<String, Vec<Decimal>>,
    pub accounts: Accounts,
    pub instruments: Instruments,
}

impl World {
    /// The first trading day on or after `date`, as an index into `days`.
    pub fn index_on(&self, date: NaiveDate) -> usize {
        match self.days.binary_search(&date) {
            Ok(i) => i,
            Err(i) => i.min(self.days.len() - 1),
        }
    }

    pub fn price(&self, security: &Security, index: usize) -> Decimal {
        self.prices[&security.id][index.min(self.days.len() - 1)]
    }

    pub fn rate(&self, index: usize) -> Decimal {
        self.fx[index.min(self.days.len() - 1)]
    }
}

/// Fills an empty portfolio. The caller has already checked that it is empty: this writes a
/// three-year history and would be nonsense on top of real data.
pub fn seed(store: &Store, portfolio: &mut Portfolio, today: NaiveDate) -> Result<()> {
    let world = build_world(store, portfolio, today)?;
    save_market(store, &world)?;
    ledger::write(store, &world)?;
    extras::write(store, portfolio, &world)?;
    Ok(())
}

fn build_world(store: &Store, portfolio: &mut Portfolio, today: NaiveDate) -> Result<World> {
    let start = today
        .checked_sub_months(Months::new(YEARS * 12))
        .unwrap_or(today)
        .with_day(1)
        .unwrap_or(today);
    let days = market::business_days(start, today);

    let euro_cash = Account::deposit("Euro broker · cash", "EUR");
    let usd_cash = Account::deposit("Global broker · cash", "USD");
    let savings = Account::deposit("Savings account", "EUR");
    store.save_account(&euro_cash)?;
    store.save_account(&usd_cash)?;
    store.save_account(&savings)?;
    let euro_depot = Account::securities("Euro broker", "EUR", &euro_cash.id);
    let usd_depot = Account::securities("Global broker", "USD", &usd_cash.id);
    store.save_account(&euro_depot)?;
    store.save_account(&usd_depot)?;

    // A group so the data-source picker has something to narrow to beyond a single account.
    store.save_account_group(&AccountGroup::new("Brokerage").with_accounts([
        euro_cash.id.clone(),
        euro_depot.id.clone(),
        usd_cash.id.clone(),
        usd_depot.id.clone(),
    ]))?;

    let fractional = dec!(0.001);
    let instruments = Instruments {
        apple: Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock),
        microsoft: Security::new("MSFT", "Microsoft Corporation", "USD", SecurityKind::Stock),
        netflix: Security::new("NFLX", "Netflix Inc.", "USD", SecurityKind::Stock),
        allworld: Security::new("VWCE", "Vanguard FTSE All-World", "EUR", SecurityKind::Etf)
            .with_quantity_step(fractional),
        core_world: Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf)
            .with_quantity_step(fractional),
        bonds: Security::new("AGGH", "iShares Global Aggregate Bond", "EUR", SecurityKind::Bond)
            .with_quantity_step(fractional),
        bitcoin: Security::new("BTC", "Bitcoin", "EUR", SecurityKind::Crypto)
            .with_quantity_step(dec!(0.00001)),
        sp500: Security::new("CSPX", "iShares Core S&P 500", "EUR", SecurityKind::Etf),
        nvidia: Security::new("NVDA", "NVIDIA Corporation", "USD", SecurityKind::Stock),
    };
    for security in instruments.all() {
        store.save_security(security)?;
    }

    portfolio.account_ids = vec![
        euro_cash.id.clone(),
        euro_depot.id.clone(),
        usd_cash.id.clone(),
        usd_depot.id.clone(),
        savings.id.clone(),
    ];
    store.save_portfolio(portfolio)?;

    // Start price, annual drift, annual volatility, how much of the market it carries, seed.
    let market = market::market_path(days.len());
    let shapes: [(&Security, market::Shape); 9] = [
        (&instruments.apple, shape(150.0, 0.15, 0.26, 1.05, 0xA9917)),
        (&instruments.microsoft, shape(255.0, 0.17, 0.23, 1.00, 0x5F702)),
        (&instruments.netflix, shape(320.0, 0.11, 0.36, 1.20, 0x4E7F1)),
        (&instruments.allworld, shape(92.0, 0.09, 0.14, 0.95, 0x07A11D)),
        (&instruments.core_world, shape(74.0, 0.09, 0.13, 0.92, 0x100D17)),
        (&instruments.bonds, shape(4.60, 0.025, 0.05, 0.18, 0xB00D51)),
        (&instruments.bitcoin, shape(26_500.0, 0.32, 0.58, 1.55, 0xB17C0)),
        (&instruments.sp500, shape(410.0, 0.12, 0.17, 1.00, 0x595001)),
        (&instruments.nvidia, shape(190.0, 0.38, 0.44, 1.35, 0x0D1A55)),
    ];
    let mut prices = BTreeMap::new();
    for (security, shape) in shapes {
        prices.insert(security.id.clone(), market::walk(&shape, &market));
    }

    let fx = market::fx_path(days.len());
    Ok(World {
        today,
        days,
        fx,
        prices,
        accounts: Accounts {
            euro_cash,
            euro_depot,
            usd_cash,
            usd_depot,
            savings,
        },
        instruments,
    })
}

fn shape(start: f64, drift: f64, vol: f64, beta: f64, seed: u64) -> market::Shape {
    market::Shape {
        start,
        drift,
        vol,
        beta,
        seed,
    }
}

/// Quotes and rates. Both are written as `manual`, and no instrument names a provider: a refresh
/// must not replace a generated series with a real one that its transactions never matched.
fn save_market(store: &Store, world: &World) -> Result<()> {
    let mut quotes = Vec::with_capacity(world.days.len() * 9);
    for security in world.instruments.all() {
        let series = &world.prices[&security.id];
        for (day, close) in world.days.iter().zip(series) {
            quotes.push(Quote {
                security_id: security.id.clone(),
                date: *day,
                close: *close,
                currency: security.currency.clone(),
                source: "manual".into(),
            });
        }
    }
    store.save_quotes(&quotes)?;

    let rates: Vec<FxRate> = world
        .days
        .iter()
        .zip(&world.fx)
        .map(|(day, rate)| FxRate::new("USD", "EUR", *day, *rate))
        .collect();
    store.save_fx_rates(&rates)?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
