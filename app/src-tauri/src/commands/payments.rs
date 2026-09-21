use crate::commands::alerts::today;
use crate::commands::{named, parse_date};
use crate::error::{UiError, UiResult};
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{
    ExpectedDividend, PaymentBucket, PaymentPeriod, PaymentRow, SecurityPaymentRow, expected_dividends,
};
use sq_core::market::DateRange;
use std::collections::HashMap;
use tauri::State;

/// One payer with the instrument named; `security_id` is null for the account's own interest.
#[derive(Debug, Serialize)]
pub struct PayerRow {
    #[serde(flatten)]
    pub row: SecurityPaymentRow,
    pub symbol: String,
    pub name: String,
}

/// The payments grid as the screen reads it: one axis, the kind rows, the payer rows and the
/// running line under them.
#[derive(Debug, Serialize)]
pub struct PaymentsData {
    pub from: String,
    pub to: String,
    pub base_currency: String,
    pub period: PaymentPeriod,
    pub buckets: Vec<PaymentBucket>,
    pub lines: Vec<PaymentRow>,
    pub payers: Vec<PayerRow>,
    pub earnings: Vec<Decimal>,
    pub cumulative: Vec<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub earnings_total: Decimal,
}

#[tauri::command]
pub fn payments_grid(
    state: State<AppState>,
    from: String,
    to: String,
    period: PaymentPeriod,
    source: Option<DataScope>,
) -> UiResult<PaymentsData> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    if to < from {
        return Err(UiError::invalid("the period ends before it starts"));
    }
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let grid = analytics
        .payments(from, to, period)
        .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    let named_by_id: HashMap<&str, (&str, &str)> = securities
        .iter()
        .map(|s| (s.id.as_str(), (s.symbol.as_str(), s.name.as_str())))
        .collect();

    Ok(PaymentsData {
        from: from.to_string(),
        to: to.to_string(),
        base_currency: analytics.base_currency().to_string(),
        period: grid.period,
        buckets: grid.buckets,
        lines: grid.lines,
        payers: grid
            .securities
            .into_iter()
            .map(|row| {
                let (symbol, name) = row
                    .security_id
                    .as_deref()
                    .and_then(|id| named_by_id.get(id))
                    .map(|(symbol, name)| ((*symbol).to_string(), (*name).to_string()))
                    .unwrap_or_default();
                PayerRow { row, symbol, name }
            })
            .collect(),
        earnings: grid.earnings,
        cumulative: grid.cumulative,
        earnings_total: grid.earnings_total,
    })
}

/// One expected dividend with the instrument named.
#[derive(Debug, Serialize)]
pub struct ExpectedDividendRow {
    #[serde(flatten)]
    pub row: ExpectedDividend,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ExpectedDividendsData {
    pub as_of: String,
    pub to: String,
    pub base_currency: String,
    pub rows: Vec<ExpectedDividendRow>,
}

/// Dividends the lens's open positions should pay over the next `months` months, at today's rates.
#[tauri::command]
pub fn dividends_expected(
    state: State<AppState>,
    months: u32,
    source: Option<DataScope>,
) -> UiResult<ExpectedDividendsData> {
    let as_of = today();
    let to = as_of
        .checked_add_months(chrono::Months::new(months.clamp(1, 24)))
        .ok_or_else(|| UiError::invalid("the forecast window runs off the calendar"))?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;
    let holdings = analytics.holdings_at(as_of)?;

    // The lag and the withholding are facts about the instrument, read where the payments
    // settle — the same reason `positions_at` reads its dividend block from the whole portfolio.
    let accounts = store.list_accounts()?;
    let unscoped;
    let payments = if scope.is_whole(&accounts) {
        &holdings
    } else {
        unscoped = scope.whole_portfolio(&store)?.holdings_at(as_of)?;
        &unscoped
    };
    let rows = expected_dividends(
        &holdings,
        &store.list_security_events()?,
        &payments.income,
        DateRange::new(as_of, to),
        analytics.base_currency(),
        &*store,
    )
    .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    Ok(ExpectedDividendsData {
        as_of: as_of.to_string(),
        to: to.to_string(),
        base_currency: analytics.base_currency().to_string(),
        rows: rows
            .into_iter()
            .map(|row| {
                let security = securities.iter().find(|s| s.id == row.security_id);
                ExpectedDividendRow {
                    symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                    name: security.map(|s| s.name.clone()).unwrap_or_default(),
                    row,
                }
            })
            .collect(),
    })
}
