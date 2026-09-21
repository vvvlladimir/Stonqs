//! What the reports screen is sent. Each row carries the core's own summary plus the names a
//! person reads it by — the core answers in ids, and an id means nothing on screen.

use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{
    ChargeRecord, ChargeSummary, DividendFrequency, DividendSummary, IncomeRecord, RealizedGain,
    RealizedSummary,
};
use sq_core::model::TransactionKind;

#[derive(Debug, Serialize)]
pub struct YearGains {
    pub year: i32,
    #[serde(flatten)]
    pub summary: RealizedSummary,
    /// Result against the cost sold, so a big year and a good year stay distinguishable.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub return_on_cost: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct SecurityGains {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(flatten)]
    pub summary: RealizedSummary,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub return_on_cost: Option<Decimal>,
}

/// One disposal as the broker made it: the ledger a tax return is actually built from.
#[derive(Debug, Serialize)]
pub struct DisposalRow {
    #[serde(flatten)]
    pub gain: RealizedGain,
    pub symbol: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub return_on_cost: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct YearDividends {
    pub year: i32,
    #[serde(flatten)]
    pub summary: DividendSummary,
}

#[derive(Debug, Serialize)]
pub struct SecurityDividends {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(flatten)]
    pub summary: DividendSummary,
    /// Lifetime dividends over the open position's cost — not a figure of the window.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub yield_on_cost: Option<Decimal>,
    /// Read off the instrument's whole payment history, like `yield_on_cost` and unlike the
    /// summary beside it: a schedule inferred from one window would name the window.
    pub frequency: DividendFrequency,
    pub last_payment: Option<String>,
    pub payments_total: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub trailing_year_base: Decimal,
    /// Last year's payments over the open position's cost: the yearly rate the position pays.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub annual_yield_on_cost: Option<Decimal>,
}

/// One dividend payment, with the broker's own currency and amount kept alongside the base one.
#[derive(Debug, Serialize)]
pub struct PaymentRow {
    #[serde(flatten)]
    pub record: IncomeRecord,
    pub symbol: String,
    pub name: String,
    pub account: String,
}

#[derive(Debug, Serialize)]
pub struct YearCharges {
    pub year: i32,
    #[serde(flatten)]
    pub summary: ChargeSummary,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
}

#[derive(Debug, Serialize)]
pub struct KindCharges {
    pub kind: TransactionKind,
    #[serde(flatten)]
    pub summary: ChargeSummary,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AccountCharges {
    pub account_id: String,
    pub account: String,
    #[serde(flatten)]
    pub summary: ChargeSummary,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
}

/// One standalone fee or tax, named the way the rest of the app names things.
#[derive(Debug, Serialize)]
pub struct ChargeRow {
    #[serde(flatten)]
    pub record: ChargeRecord,
    pub symbol: String,
    pub name: String,
    pub account: String,
}

/// Everything the Reports screen shows for one window, plus the window before it so a
/// tile can say which way a number moved. Every figure is filtered to `[from, to]`;
/// `yield_on_cost` is the one lifetime exception and says so on its field.
#[derive(Debug, Serialize)]
pub struct ReportsData {
    pub from: String,
    pub to: String,
    pub previous_from: String,
    pub previous_to: String,
    pub base_currency: String,

    pub gains_by_year: Vec<YearGains>,
    pub gains_by_security: Vec<SecurityGains>,
    pub gains_total: RealizedSummary,
    pub gains_previous: RealizedSummary,
    /// Movement against the previous window, so a tile never subtracts two money strings.
    #[serde(with = "rust_decimal::serde::str")]
    pub gains_change_base: Decimal,
    /// Result of the whole window against the cost it came from.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub gains_return_on_cost: Option<Decimal>,
    pub disposals: Vec<DisposalRow>,

    pub dividends_by_year: Vec<YearDividends>,
    pub dividends_by_security: Vec<SecurityDividends>,
    pub dividends_total: DividendSummary,
    pub dividends_previous: DividendSummary,
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_change_base: Decimal,
    pub payments: Vec<PaymentRow>,

    /// Fees and taxes recorded as separate transactions.
    pub charges_by_year: Vec<YearCharges>,
    pub charges_by_kind: Vec<KindCharges>,
    pub charges_by_account: Vec<AccountCharges>,
    pub charges_total: ChargeSummary,
    pub charges_previous: ChargeSummary,
    #[serde(with = "rust_decimal::serde::str")]
    pub charges_change_base: Decimal,
    /// Fees plus taxes: the number the "Total costs" tile shows.
    #[serde(with = "rust_decimal::serde::str")]
    pub charges_total_base: Decimal,
    pub charge_rows: Vec<ChargeRow>,
}
