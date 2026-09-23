//! # sq-core
//!
//! Investment-portfolio core: model, SQLite storage, quote/FX providers,
//! performance math. Library only — no UI, no Tauri, no printing or
//! logging, so it's reusable (CLI, desktop, import scripts) and testable
//! standalone via `cargo test`. See repo root `CLAUDE.md` for the full
//! module/dependency layout.
//!
//! ## Example
//!
//! ```
//! use chrono::NaiveDate;
//! use sq_core::calc::{build_holdings, value_holdings};
//! use sq_core::fx::{FxRate, RateLookup};
//! use sq_core::market::PriceLookup;
//! use sq_core::model::{Account, Security, SecurityKind, Transaction};
//! use sq_core::storage::Store;
//! use rust_decimal_macros::dec;
//!
//! let store = Store::open_in_memory()?;
//! // Cash lives on a deposit account, securities on a depot backed by it.
//! let cash = Account::deposit("Bank", "EUR");
//! store.save_account(&cash)?;
//! let account = Account::securities("Broker", "EUR", &cash.id);
//! let apple = Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock);
//! store.save_account(&account)?;
//! store.save_security(&apple)?;
//!
//! let day = NaiveDate::from_ymd_opt(2024, 6, 5).unwrap();
//! let buy = Transaction::buy(&account.id, &apple.id, day, dec!(5), dec!(195), "USD")
//!     .with_fx_from_base_total(dec!(900)); // 5x195 USD actually cost 900 EUR
//! store.save_transaction(&buy)?;
//!
//! store.save_fx_rates(&[FxRate::new("USD", "EUR", day, dec!(0.93))])?;
//! store.save_quotes(&[sq_core::market::Quote {
//!     security_id: apple.id.clone(),
//!     date: day,
//!     close: dec!(200),
//!     currency: "USD".into(),
//!     source: "manual".into(),
//! }])?;
//!
//! let txs = store.transactions_for_account(&account.id)?;
//! let holdings = build_holdings(&txs, "EUR", &store)?;
//! let valuation = value_holdings(&holdings, "EUR", day, &store, &store)?;
//! // 5 x 200 USD x 0.93 = 930 EUR of securities; cost basis was 900 EUR.
//! assert_eq!(valuation.securities_value_base, dec!(930.00));
//! # Ok::<(), sq_core::Error>(())
//! ```

// A bare `unwrap()` outside tests is a crash in somebody's portfolio. An invariant that really
// cannot fail is written as `expect("why")`, so the reason survives into the panic message.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod calc;
pub mod error;
pub mod fx;
pub mod import;
pub mod inflation;
pub mod market;
pub mod model;
pub mod money;
pub mod sources;
pub mod storage;

pub use error::{Error, Result};

/// Commonly used types in one `use` — callers typically touch most modules at once.
pub mod prelude {
    pub use crate::calc::{
        AlertStatus, Allocation, AllocationBucket, BenchmarkComparison, CalculationSheet, Contribution,
        DividendProfile, DividendSummary, Holdings, InstrumentMove, NearestLevel, Peak, Period, PeriodPreset,
        PlanOccurrence, PlannedTrade, PortfolioAnalytics, PortfolioValuation, RealizedSummary,
        RebalanceOptions, RebalancePlan, RiskMetrics, Trade, TradeStats, ValueSeries, alert_status,
        all_time_high, build_holdings, capital_gains_by_year, closed_trades, contribution_schedule,
        contributions_by_month, dividend_profiles, dividends_by_year, due_occurrences, investable_amount,
        open_trades, plan_occurrence, plan_transactions, risk_metrics, trade_stats, value_holdings,
        value_series, xirr, yield_on_cost,
    };
    pub use crate::error::{Error, Result};
    pub use crate::fx::{EcbProvider, FxRate, FxService, RateCache, RateLookup, StaticFxProvider};
    pub use crate::market::{
        DateRange, Listing, ListingDirectory, MarketDataService, OpenFigiDirectory, PriceCache, PriceLookup,
        PricePoint, Quote, QuoteProvider, StooqProvider, YahooProvider,
    };
    pub use crate::model::{
        Account, AccountGroup, AccountKind, AlertCrossing, AlertDirection, AlertKind, AlertSide,
        AllocationTarget, AttributeKind, CashClassification, ContributionLimit, CorporateAction,
        CorporateActionKind, CostBasisMethod, CrossingDirection, Goal, Interval, InvestmentPlan, PlanLeg,
        Portfolio, Position, Schedule, Security, SecurityAlert, SecurityAttributeDef, SecurityClassification,
        SecurityEvent, SecurityEventKind, SecurityKind, Taxonomy, TaxonomyKind, TaxonomyNode, Transaction,
        TransactionKind, Watchlist, observed_quantity_step,
    };
    pub use crate::money::{Currency, Money};
    pub use crate::sources;
    pub use crate::storage::{AttributeValues, Store};
}
