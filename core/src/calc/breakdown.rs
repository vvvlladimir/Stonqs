//! The calculation sheet: how the opening value, the flows and what was earned add up to the
//! closing one, one row per calendar chunk.
//!
//! It introduces no new number. Every column is an existing figure narrowed to one chunk, and
//! the return column *is* [`returns_by_period`] — a second implementation that disagreed with the
//! TWR shown beside it by a hundredth of a percent would destroy the trust the sheet exists to
//! build.

use super::{
    ChargeSummary, Holdings, Period, PeriodSummary, ValueSeries, costs_paid_over, period_summary,
    returns_by_period,
};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::market::DateRange;
use crate::model::Transaction;
use crate::money::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One chunk of the period: what it opened with, what moved, and what it closed at.
///
/// The identity every row keeps is `end = start + flow + delta`, and `delta` is split into the
/// three things that can produce it: the market, the income it paid, and what it cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalculationRow {
    pub from: chrono::NaiveDate,
    pub to: chrono::NaiveDate,
    /// Value carried in: the previous row's close, and for the first row the value before the
    /// window opened — so a deposit made on day one is money paid in, not an opening balance.
    #[serde(with = "rust_decimal::serde::str")]
    pub start_value_base: Decimal,
    /// Net external flow inside the chunk; `+` is money brought in.
    #[serde(with = "rust_decimal::serde::str")]
    pub external_flow_base: Decimal,
    /// Dividends and interest credited, gross — the tax withheld from them is in `costs_base`.
    #[serde(with = "rust_decimal::serde::str")]
    pub income_base: Decimal,
    /// Fees and taxes actually paid, trade commissions included (see [`super::costs_paid`]).
    pub costs: ChargeSummary,
    /// The two above added up: money is summed where it is a `Decimal`, never on the wire.
    #[serde(with = "rust_decimal::serde::str")]
    pub costs_base: Decimal,
    /// What is left of the change once income and costs are named: the market's own work.
    #[serde(with = "rust_decimal::serde::str")]
    pub market_change_base: Decimal,
    /// The change with the flows taken out — what the chunk earned.
    #[serde(with = "rust_decimal::serde::str")]
    pub delta_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub end_value_base: Decimal,
    /// The chunk's time-weighted return, as a fraction.
    #[serde(with = "rust_decimal::serde::str")]
    pub twr: Decimal,
    /// The chunks so far, chained: the last row's value is the period's own TWR.
    #[serde(with = "rust_decimal::serde::str")]
    pub cumulative_twr: Decimal,
}

/// The sheet: its rows, the totals they add up to, and the return they chain to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalculationSheet {
    pub base_currency: Currency,
    pub period: Period,
    pub rows: Vec<CalculationRow>,
    /// The same money view the performance screen shows, so the footer can be checked against it.
    pub total: PeriodSummary,
    /// The period's TWR, computed from the daily series rather than from the rows.
    #[serde(with = "rust_decimal::serde::str")]
    pub twr: Decimal,
}

/// Builds the sheet from one daily series, splitting it on calendar boundaries.
///
/// `holdings` supplies the income already recorded, `transactions` the costs; both are the same
/// ones every other figure on the performance screen is read from.
pub fn calculation_sheet(
    series: &ValueSeries,
    holdings: &Holdings,
    transactions: &[Transaction],
    rates: &dyn RateLookup,
    period: Period,
) -> Result<CalculationSheet> {
    let base = series.base_currency.clone();
    let empty = CalculationSheet {
        base_currency: base.clone(),
        period,
        rows: Vec::new(),
        total: period_summary(series),
        twr: Decimal::ZERO,
    };
    let (Some(from), Some(to)) = (series.first_date(), series.last_date()) else {
        return Ok(empty);
    };

    let ranges = period.split(DateRange::new(from, to));
    let returns = returns_by_period(series, period);
    let costs = costs_paid_over(transactions, &base, &ranges, rates)?;

    // A window opens on what was there before it (ADR-0043): day one's own flow is money paid
    // in, which at inception makes the opening zero rather than the first purchase.
    let mut start = series.total_value_base.first().copied().unwrap_or_default()
        - series.external_flow_base.first().copied().unwrap_or_default();
    let mut chained = Decimal::ONE;
    let mut rows = Vec::with_capacity(ranges.len());

    for (i, range) in ranges.iter().enumerate() {
        let end = series.value_on(range.to).unwrap_or(start);
        let external_flow_base = sum_flows(series, *range);
        let income_base: Decimal = holdings
            .income
            .iter()
            .filter(|r| r.date >= range.from && r.date <= range.to)
            .map(|r| r.gross_base)
            .sum();
        let costs = costs.get(i).cloned().unwrap_or_default();
        let delta_base = end - start - external_flow_base;
        let twr = returns.get(i).map(|r| r.twr).unwrap_or_default();
        chained *= Decimal::ONE + twr;

        rows.push(CalculationRow {
            from: range.from,
            to: range.to,
            start_value_base: start,
            external_flow_base,
            income_base,
            market_change_base: delta_base - income_base + costs.total_base(),
            costs_base: costs.total_base(),
            costs,
            delta_base,
            end_value_base: end,
            twr,
            cumulative_twr: chained - Decimal::ONE,
        });
        start = end;
    }

    Ok(CalculationSheet {
        rows,
        twr: series.twr()?,
        ..empty
    })
}

/// Flows inside one chunk. The first day of the whole series counts too — it is the day the
/// window opened on, and [`period_summary`] counts it for the same reason.
fn sum_flows(series: &ValueSeries, range: DateRange) -> Decimal {
    series
        .dates
        .iter()
        .zip(series.external_flow_base.iter())
        .filter(|(date, _)| **date >= range.from && **date <= range.to)
        .map(|(_, flow)| *flow)
        .sum()
}
