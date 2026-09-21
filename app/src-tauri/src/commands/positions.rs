use crate::commands::named;
use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::scope::DataScope;
use crate::state::AppState;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{
    CostBasisRow, DividendFrequency, PositionRisk, day_changes, dividend_profiles, dividend_yield, peak_of,
    value_holdings, yield_on_cost,
};
use sq_core::market::DateRange;
use sq_core::model::Lot;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct PositionRow {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    pub currency: String,
    pub cost_currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fx_rate: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub market_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub unrealized_pnl_base: Decimal,
    /// The exchange rate's share of `unrealized_pnl_base`; zero in a single-currency portfolio.
    #[serde(with = "rust_decimal::serde::str")]
    pub currency_gain_base: Decimal,
    /// The instrument's own share: `unrealized_pnl_base` minus the currency's.
    #[serde(with = "rust_decimal::serde::str")]
    pub instrument_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_base: Decimal,
    /// Payment schedule read off this instrument's own history.
    pub dividend_frequency: DividendFrequency,
    pub dividend_payments: usize,
    pub dividend_last: Option<String>,
    /// Paid over the last year, and that amount against today's value and against the cost.
    #[serde(with = "rust_decimal::serde::str")]
    pub dividend_year_base: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub dividend_yield: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub yield_on_cost: Option<Decimal>,
    /// Highest close ever stored for this instrument, in its quote currency, and how far
    /// below it the current price stands. `None` when no quotes are stored.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub ath_price: Option<Decimal>,
    pub ath_date: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub ath_distance: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub previous_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub day_change_base: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub day_change: Option<Decimal>,
    pub accounts: Vec<PositionAccount>,
    pub lots: Vec<Lot>,
}

#[derive(Debug, Serialize)]
pub struct PositionAccount {
    pub account_id: String,
    pub account_name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
}

#[derive(Debug, Serialize)]
pub struct PositionsData {
    pub date: String,
    pub base_currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub day_change_base: Decimal,
    pub rows: Vec<PositionRow>,
}

#[tauri::command]
pub fn positions_at(
    state: State<AppState>,
    date: String,
    source: Option<DataScope>,
) -> UiResult<PositionsData> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let holdings = analytics.holdings_at(date)?;
    let valuation = value_holdings(&holdings, analytics.base_currency(), date, &*store, &*store)
        .map_err(|e| named(&store, e))?;

    let changes = day_changes(&holdings, analytics.base_currency(), date, &*store, &*store)
        .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    let accounts = store.list_accounts()?;
    let total = valuation.total_value_base;

    // A dividend settles on the deposit account, so a depot-only scope drops every payment and
    // a schedule read from it would call a quarterly payer "never paid". What an instrument has
    // paid is a fact about the instrument, not about the lens, so the whole dividend block comes
    // from the whole portfolio; value, result and weight beside it stay the scope's.
    let unscoped;
    let payments = if scope.is_whole(&accounts) {
        &holdings
    } else {
        unscoped = scope.whole_portfolio(&store)?.holdings_at(date)?;
        &unscoped
    };
    // One profile per payer, from the whole history: a schedule read off a window names the
    // window instead of the instrument.
    let profiles = dividend_profiles(&payments.income, date);

    let rows = valuation
        .positions
        .iter()
        .map(|p| {
            let security = securities.iter().find(|s| s.id == p.security_id);
            let position = holdings.positions.get(&p.security_id);
            let change = changes.positions.get(&p.security_id);
            let dividends = profiles.get(&p.security_id).cloned().unwrap_or_default();
            let peak = quote_peak(&store, &p.security_id, date);
            PositionRow {
                security_id: p.security_id.clone(),
                symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                currency: p.currency.clone(),
                cost_currency: p.cost_currency.clone(),
                quantity: p.quantity,
                price: p.price,
                fx_rate: p.fx_rate,
                market_value_base: p.market_value_base,
                cost_basis_base: p.cost_basis_base,
                unrealized_pnl_base: p.unrealized_pnl_base,
                currency_gain_base: p.currency_gain_base,
                instrument_gain_base: p.instrument_gain_base(),
                realized_pnl_base: p.realized_pnl_base,
                dividends_base: payments
                    .positions
                    .get(&p.security_id)
                    .map(|x| x.dividends_base)
                    .unwrap_or_default(),
                dividend_frequency: dividends.frequency,
                dividend_payments: dividends.payments,
                dividend_last: dividends.last_payment.map(|d| d.to_string()),
                dividend_yield: dividend_yield(dividends.trailing_year_base, p.market_value_base),
                yield_on_cost: yield_on_cost(payments, &p.security_id),
                dividend_year_base: dividends.trailing_year_base,
                ath_price: peak.as_ref().map(|peak| peak.value),
                ath_date: peak.as_ref().map(|peak| peak.date.to_string()),
                ath_distance: peak.as_ref().and_then(|peak| peak.distance),
                weight: if total.is_zero() {
                    Decimal::ZERO
                } else {
                    p.market_value_base / total
                },
                previous_price: change.map(|c| c.previous_price),
                day_change_base: change.map(|c| c.change_base),
                day_change: change.map(|c| c.change),
                accounts: position
                    .map(|x| {
                        x.accounts
                            .iter()
                            .map(|(id, quantity)| PositionAccount {
                                account_name: accounts
                                    .iter()
                                    .find(|a| &a.id == id)
                                    .map(|a| a.name.clone())
                                    .unwrap_or_default(),
                                account_id: id.clone(),
                                quantity: *quantity,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                lots: position.map(|x| x.lots.clone()).unwrap_or_default(),
            }
        })
        .collect();

    Ok(PositionsData {
        date: date.to_string(),
        base_currency: valuation.base_currency,
        total_value_base: total,
        day_change_base: changes.total_base,
        rows,
    })
}

/// Highest stored close for one instrument, in its own quote currency. Stored quotes are the
/// only history there is, so an instrument whose quotes start in 2023 has a 2023 high and says
/// so through `ath_date`. A read failure is an absent high, not a failed screen.
fn quote_peak(
    store: &sq_core::storage::Store,
    security_id: &str,
    date: NaiveDate,
) -> Option<sq_core::calc::Peak> {
    let coverage = store.quote_coverage(security_id).ok()??;
    let quotes = store
        .quotes_in_range(security_id, DateRange::new(coverage.from, date))
        .ok()?;
    peak_of(quotes.into_iter().map(|q| (q.date, q.close)))
}

#[derive(Debug, Serialize)]
pub struct PositionReturn {
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub xirr: Option<Decimal>,
}

#[tauri::command]
pub fn position_return(
    state: State<AppState>,
    security_id: String,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<PositionReturn> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    Ok(PositionReturn {
        twr: unsolvable_to_none(analytics.position_twr(&security_id, from, to))?,
        xirr: unsolvable_to_none(analytics.position_xirr(&security_id, to))?,
    })
}

#[derive(Debug, Serialize)]
pub struct PositionReturnRow {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr_annualized: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub xirr: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl_base: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub absolute_performance: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub contribution: Decimal,
    pub risk: Option<PositionRisk>,
}

#[tauri::command]
pub fn position_returns(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<Vec<PositionReturnRow>> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let rows = scope
        .analytics(&store)?
        .position_returns(from, to)
        .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let security = securities.iter().find(|s| s.id == r.security_id);
            PositionReturnRow {
                symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                security_id: r.security_id,
                twr: r.twr,
                twr_annualized: r.twr_annualized,
                xirr: r.xirr,
                pnl_base: r.pnl_base,
                absolute_performance: r.absolute_performance,
                fees_base: r.fees_base,
                taxes_base: r.taxes_base,
                contribution: r.contribution,
                risk: r.risk,
            }
        })
        .collect())
}

/// One instrument's purchase value under both cost-basis methods. Which one the portfolio
/// itself answers to is not sent: both columns are named after their method, so neither's
/// meaning moves with a setting.
#[derive(Debug, Serialize)]
pub struct PositionCostRow {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(flatten)]
    pub row: CostBasisRow,
}

#[derive(Debug, Serialize)]
pub struct PositionCostData {
    pub date: String,
    pub base_currency: String,
    pub rows: Vec<PositionCostRow>,
}

/// Its own command rather than more fields on `positions_at`: it costs a second holdings pass,
/// and a screen showing neither column must not pay for it.
#[tauri::command]
pub fn positions_cost_basis(
    state: State<AppState>,
    date: String,
    source: Option<DataScope>,
) -> UiResult<PositionCostData> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;
    let rows = analytics
        .cost_basis_comparison(date)
        .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    Ok(PositionCostData {
        date: date.to_string(),
        base_currency: analytics.base_currency().to_string(),
        rows: rows
            .into_iter()
            .map(|row| {
                let security = securities.iter().find(|s| s.id == row.security_id);
                PositionCostRow {
                    symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                    name: security.map(|s| s.name.clone()).unwrap_or_default(),
                    security_id: row.security_id.clone(),
                    row,
                }
            })
            .collect(),
    })
}

fn unsolvable_to_none(result: sq_core::Result<Decimal>) -> UiResult<Option<Decimal>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(sq_core::Error::Math(_)) => Ok(None),
        Err(e) => Err(UiError::from(e)),
    }
}
