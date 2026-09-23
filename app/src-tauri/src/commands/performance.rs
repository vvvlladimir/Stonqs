use crate::commands::named;
use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{
    CalculationSheet, ChargeSummary, GrowthSeries, Peak, Period, PeriodReturn, PeriodSummary, RiskReport,
    TradingVolume, ValueSeries, all_time_high,
};
use sq_core::market::DateRange;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct PerformanceData {
    pub from: String,
    pub to: String,
    pub base_currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub twr: Decimal,
    /// The same return as a yearly rate; `None` for a period shorter than a day.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr_annualized: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub xirr: Option<Decimal>,
    /// Values, flows, and the capital they add up to — the money side of the same period.
    pub summary: PeriodSummary,
    /// Fees and taxes paid in the period, trade commissions included.
    pub costs: ChargeSummary,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub fee_rate: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub tax_rate: Option<Decimal>,
    /// Bought and sold in the period, and that volume against the capital at work.
    pub volume: TradingVolume,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub turnover_rate: Option<Decimal>,
    /// The period's highest value and how far below it the period ends; `None` for an
    /// empty portfolio, which has no high to stand under.
    pub peak: Option<Peak>,
    pub series: ValueSeries,
    pub growth: GrowthSeries,
    pub monthly_returns: Vec<PeriodReturn>,
    pub annual_returns: Vec<PeriodReturn>,
}

#[tauri::command]
pub fn performance_summary(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<PerformanceData> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let series = analytics.series(range).map_err(|e| named(&store, e))?;
    let growth = series.growth();
    let monthly_returns = sq_core::calc::returns_by_period(&series, Period::Month);
    let annual_returns = sq_core::calc::returns_by_period(&series, Period::Year);

    let summary = sq_core::calc::period_summary(&series);
    let costs = analytics
        .costs(range.from, range.to)
        .map_err(|e| named(&store, e))?;
    let twr = analytics.twr(range.from, range.to)?;
    let volume = analytics.trading_volume(range.from, range.to)?;

    Ok(PerformanceData {
        from: range.from.to_string(),
        to: range.to.to_string(),
        base_currency: analytics.base_currency().to_string(),
        twr,
        twr_annualized: sq_core::calc::annualize(twr, range.from, range.to),
        xirr: match analytics.xirr(range.to) {
            Ok(v) => Some(v),
            Err(sq_core::Error::Math(_)) => None,
            Err(e) => return Err(UiError::from(e)),
        },
        fee_rate: summary.rate_of(costs.fees_base),
        tax_rate: summary.rate_of(costs.taxes_base),
        turnover_rate: summary.rate_of(volume.volume_base),
        volume,
        peak: all_time_high(&series),
        summary,
        costs,
        growth,
        monthly_returns,
        annual_returns,
        series,
    })
}

/// The calculation sheet of the period: one row per calendar chunk, showing how the opening
/// value, the flows and what was earned add up to the closing one. The granularity is the
/// screen's, not the core's — a decade reads by year and a month by day.
#[tauri::command]
pub fn performance_breakdown(
    state: State<AppState>,
    from: String,
    to: String,
    period: Period,
    source: Option<DataScope>,
) -> UiResult<CalculationSheet> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .calculation_sheet(range, period)
        .map_err(|e| named(&store, e))
}

/// The calculation sheet as a CSV of what the panel shows, written where the user points.
#[tauri::command]
pub fn performance_sheet_save(
    state: State<AppState>,
    from: String,
    to: String,
    period: Period,
    path: String,
) -> UiResult<()> {
    use crate::commands::reports::csv::{money, row};

    let sheet = performance_breakdown(state, from, to, period, None)?;
    let c = &sheet.base_currency;
    let mut csv = row([
        "from",
        "to",
        &format!("opening {c}"),
        &format!("flows {c}"),
        &format!("market {c}"),
        &format!("income {c}"),
        &format!("costs {c}"),
        &format!("result {c}"),
        &format!("closing {c}"),
        "return",
        "chained return",
    ]);
    for r in &sheet.rows {
        csv.push_str(&row([
            r.from.to_string().as_str(),
            r.to.to_string().as_str(),
            money(r.start_value_base).as_str(),
            money(r.external_flow_base).as_str(),
            money(r.market_change_base).as_str(),
            money(r.income_base).as_str(),
            money(r.costs_base).as_str(),
            money(r.delta_base).as_str(),
            money(r.end_value_base).as_str(),
            money(r.twr).as_str(),
            money(r.cumulative_twr).as_str(),
        ]));
    }
    csv.push_str(&row([
        sheet
            .rows
            .first()
            .map(|r| r.from.to_string())
            .unwrap_or_default()
            .as_str(),
        sheet
            .rows
            .last()
            .map(|r| r.to.to_string())
            .unwrap_or_default()
            .as_str(),
        money(sheet.total.start_value_base).as_str(),
        money(sheet.total.net_flow_base).as_str(),
        "",
        "",
        "",
        money(sheet.total.delta_base).as_str(),
        money(sheet.total.end_value_base).as_str(),
        "",
        money(sheet.twr).as_str(),
    ]));

    // Excel reads a semicolon-separated file as UTF-8 only when it opens with a BOM.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

#[tauri::command]
pub fn risk_report(
    state: State<AppState>,
    from: String,
    to: String,
    risk_free_rate: f64,
    window_days: usize,
    source: Option<DataScope>,
) -> UiResult<RiskReport> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .risk_report(range, risk_free_rate, window_days)
        .map_err(|e| named(&store, e))
}

#[tauri::command]
pub fn benchmark_compare(
    state: State<AppState>,
    security_id: String,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<sq_core::calc::BenchmarkComparison> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .benchmark(&security_id, range.from, range.to)
        .map_err(|e| named(&store, e))
}

#[tauri::command]
pub fn benchmark_series(
    state: State<AppState>,
    security_id: String,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<GrowthSeries> {
    let range = date_range(&from, &to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let dates: Vec<chrono::NaiveDate> = std::iter::successors(Some(range.from), |d| {
        d.succ_opt().filter(|next| *next <= range.to)
    })
    .collect();
    scope
        .analytics(&store)?
        .benchmark_growth(&security_id, &dates)
        .map_err(|e| named(&store, e))
}

pub(crate) fn date_range(from: &str, to: &str) -> UiResult<DateRange> {
    let from = parse_date(from)?;
    let to = parse_date(to)?;
    if to < from {
        return Err(UiError::invalid("the period ends before it starts"));
    }
    Ok(DateRange::new(from, to))
}
