//! Calculations from hand-worked examples.

mod breakdown;
mod charges;
mod forecast;
mod income;
mod returns;
mod trades;
mod valuation;

// `support` is shared by every integration binary, so it stays one directory up.
#[path = "../support/mod.rs"]
mod support;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use sq_core::calc::costs_paid;
use sq_core::calc::{
    Assignment, CashFlow, DividendFrequency, FireAssumptions, HoldingsOptions, IncomeNode, PaymentLine,
    PaymentPeriod, Period, UNCLASSIFIED_KEY, all_time_high, build_holdings, build_holdings_with,
    calculation_sheet, capital_gains_by_year, capital_gains_total, charges_between, charges_by_account,
    charges_by_kind, charges_by_security, charges_by_year, charges_total, closed_trades, compare_cost_basis,
    day_changes, dividend_profiles, dividends_by_year, dividends_total, fire_projection, income_between,
    income_by_kind, income_by_month, income_by_month_of_year, income_by_security, income_by_taxonomy,
    income_by_year, income_by_year_kind, income_of_kind, income_total, open_trades, payment_grid,
    period_summary, position_returns, realized_between, risk_metrics, trade_stats, trading_volume,
    transactions_net_by_month, transactions_net_by_year, twr_between, valuation_at, value_holdings,
    value_series, xirr,
};
use sq_core::market::DateRange;
use sq_core::model::{CostBasisMethod, TaxonomyNode, Transaction, TransactionKind, cash_subject_key};
use support::{FakePrices, FakeRates, d};

const ACC: &str = "acc-1";

const AAPL: &str = "sec-aapl";
