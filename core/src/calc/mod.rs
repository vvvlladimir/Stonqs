//! Transactions -> [`Holdings`] -> [`PortfolioValuation`] -> metrics (TWR, XIRR).
//! `calc` depends only on [`crate::market::PriceLookup`] / [`crate::fx::RateLookup`], never on SQLite or the network.

mod alerts;
mod allocation;
mod balances;
mod benchmark;
mod capital_gains;
mod charges;
mod cost_basis;
mod dividend_forecast;
mod dividends;
mod engine;
mod fire;
mod holdings;
mod income;
mod inflation;
mod journal;
mod payments;
mod peak;
mod periods;
mod plans;
mod rebalance;
mod risk;
mod scope;
mod series;
mod summary;
mod trades;
mod twr;
mod valuation;
mod watchlist;
mod xirr;

pub use alerts::{AlertCheck, AlertStatus, alert_status, check_alert, crossing_close};
pub use allocation::{
    Allocation, AllocationBucket, Assignment, CASH_KEY, CashSubject, MemberScope, NodeMember, SubjectKind,
    TaxonomySubject, UNCLASSIFIED_KEY, allocation_by_account, allocation_by_currency, allocation_by_security,
    allocation_by_taxonomy, allocation_members, allocation_tree, taxonomy_subjects,
};
pub use balances::{cash_balances, settlement_accounts};
pub use benchmark::{
    BenchmarkComparison, benchmark_return, benchmark_series, benchmark_start, compare_to_benchmark,
};
pub use capital_gains::{
    RealizedSummary, capital_gains_by_security, capital_gains_by_year, capital_gains_by_year_and_security,
    capital_gains_total, realized_between, return_on_cost,
};
pub use charges::{
    ChargeSummary, charges_between, charges_by_account, charges_by_kind, charges_by_security,
    charges_by_year, charges_total, costs_paid, costs_paid_by_security,
};
pub use cost_basis::{CostBasisFigures, CostBasisRow, compare_cost_basis};
pub use dividend_forecast::{ExpectedDividend, expected_dividends};
pub use dividends::{
    DividendFrequency, DividendProfile, DividendSummary, dividend_profile, dividend_profiles,
    dividend_records, dividend_yield, dividends_by_security, dividends_by_year, dividends_total,
    yield_on_cost,
};
pub use engine::{
    PortfolioAnalytics, PositionReturnRow, PositionRisk, RealPerformance, holdings_at, position_returns,
    position_twr_between, position_xirr, twr_between, twr_between_with, valuation_at, valuation_at_with,
};
pub use fire::{FireAssumptions, FireProjection, fire_projection, percent_to_rate};
pub use holdings::{
    CashFlow, ChargeRecord, Holdings, HoldingsOptions, IncomeRecord, RealizedGain, build_holdings,
    build_holdings_with,
};
pub(crate) use holdings::{HoldingsBuilder, ordered_events, resolve_rate};
pub use income::{
    IncomeNode, IncomeSummary, TaxonomyIncome, income_between, income_by_kind, income_by_month,
    income_by_month_of_year, income_by_security, income_by_taxonomy, income_by_year, income_by_year_kind,
    income_of_kind, income_total,
};
pub use inflation::{
    RealReturn, deflation_end, inflation_factor, inflation_series, real_period_return, real_return, real_xirr,
};
pub use journal::{
    MonthlyNet, YearlyNet, transaction_amount_base, transaction_net_base, transactions_net_by_month,
    transactions_net_by_year,
};
pub use payments::{
    PaymentBucket, PaymentGrid, PaymentLine, PaymentPeriod, PaymentRow, SecurityPaymentRow, payment_grid,
};
pub use peak::{Peak, all_time_high, peak_of};
pub use periods::{Period, PeriodPreset, PeriodReturn, PeriodSpec, returns_by_period};
pub use plans::{
    Contribution, PlanOccurrence, PlannedTrade, contribution_schedule, contributions_by_month,
    due_occurrences, investable_amount, monthly_contribution, plan_occurrence, plan_transactions,
};
pub use rebalance::{CashDeposit, RebalanceItem, RebalanceOptions, RebalancePlan, RebalanceTrade, rebalance};
pub use risk::{
    Drawdown, RiskMetrics, RiskReport, TRADING_DAYS_PER_YEAR, drawdown_series, drawdowns, return_series,
    risk_metrics, risk_report, rolling_volatility,
};
pub use scope::scoped_transactions;
pub(crate) use series::dietz_capital;
pub use series::{GrowthSeries, StatSeries, ValueSeries, value_series};
pub use summary::{PeriodSummary, period_summary};
pub use trades::{
    Trade, TradeBook, TradeStats, TradingVolume, closed_trades, open_trades, trade_stats, trading_volume,
};
pub use twr::{TwrPoint, annualize, time_weighted_return};
pub use valuation::{
    DayChange, DayChanges, PortfolioValuation, PositionValuation, day_changes, value_holdings,
};
pub use watchlist::{InstrumentMove, NearestLevel, instrument_move, nearest_level};
pub use xirr::{XIRR_MAX_ITERATIONS, xirr};
