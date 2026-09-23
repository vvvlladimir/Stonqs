//! Time-weighted and money-weighted return, for the portfolio and for one position.
//!
//! TWR splits the window at every external cash flow, so it measures the decision rather than
//! the timing of the deposits; XIRR answers the other question and is not comparable to it.

use crate::calc::risk::metrics_from_returns;
use crate::calc::{
    CalculationSheet, CashFlow, ChargeSummary, Period, PeriodSummary, calculation_sheet, costs_paid,
    period_summary,
};
use crate::calc::{
    Holdings, HoldingsBuilder, HoldingsOptions, PortfolioValuation, TwrPoint, ValueSeries, annualize,
    build_holdings_with, costs_paid_by_security, dietz_capital, ordered_events, resolve_rate,
    time_weighted_return, value_holdings, value_series, xirr,
};
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::{DateRange, PriceLookup};
use crate::model::{Transaction, TransactionKind};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{PortfolioAnalytics, holdings_at, valuation_at_with};

/// Computes TWR for `(from, to]`, splitting the range on the day before each flow.
/// Values are fetched only at boundaries; flows on `from` are part of the baseline.
pub fn twr_between(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    twr_between_with(
        transactions,
        base,
        from,
        to,
        prices,
        rates,
        HoldingsOptions::default(),
    )
}

/// Computes TWR with explicit holdings options.
pub fn twr_between_with(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Decimal> {
    let all = build_holdings_with(transactions, base, rates, options)?;

    let mut breaks: BTreeSet<NaiveDate> = BTreeSet::new();
    breaks.insert(from);
    breaks.insert(to);
    for flow in &all.external_flows {
        let boundary = flow.date - Duration::days(1);
        if boundary > from && boundary < to {
            breaks.insert(boundary);
        }
    }
    let breaks: Vec<NaiveDate> = breaks.into_iter().collect();

    let mut points = Vec::with_capacity(breaks.len());
    for (i, date) in breaks.iter().enumerate() {
        let value = valuation_at_with(transactions, base, *date, prices, rates, options)?.total_value_base;
        // Include flows after the previous boundary and through this boundary.
        let external_flow = if i == 0 {
            Decimal::ZERO
        } else {
            let (prev, cur) = (breaks[i - 1], *date);
            all.external_flows
                .iter()
                .filter(|f| f.date > prev && f.date <= cur)
                .map(|f| f.amount_base)
                .sum()
        };
        points.push(TwrPoint {
            date: *date,
            end_value: value,
            external_flow,
        });
    }

    time_weighted_return(&points)
}

/// Transactions for one security, including its dividends.
fn transactions_for_security(transactions: &[Transaction], security_id: &str) -> Vec<Transaction> {
    transactions
        .iter()
        .filter(|t| t.security_id.as_deref() == Some(security_id))
        .cloned()
        .collect()
}

/// Values one position at a date in base currency.
fn position_value(
    holdings: &Holdings,
    security_id: &str,
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    let Some(position) = holdings.positions.get(security_id) else {
        return Ok(Decimal::ZERO);
    };
    if position.is_closed() {
        return Ok(Decimal::ZERO);
    }
    let price = prices
        .price_as_of(security_id, date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "price",
            key: security_id.to_string(),
            date,
        })?;
    // Convert the quote currency, matching `value_holdings`.
    let rate = rates
        .rate_as_of(&price.currency, base, date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{}/{}", price.currency, base),
            date,
        })?;
    Ok(position.quantity * price.close * rate)
}

/// Position cash flows in base currency, using portfolio flow signs.
fn position_flows(transactions: &[Transaction], base: &str, rates: &dyn RateLookup) -> Result<Vec<CashFlow>> {
    let mut flows = Vec::new();
    for t in transactions {
        let rate = resolve_rate(t, base, rates)?;
        let gross = t.gross_in_transaction_currency() * rate;
        let amount = match t.kind {
            TransactionKind::Buy | TransactionKind::DeliveryInbound => gross,
            TransactionKind::Sell | TransactionKind::DeliveryOutbound | TransactionKind::Dividend => -gross,
            // Transfers between own accounts do not change the position.
            _ => continue,
        };
        flows.push(CashFlow {
            date: t.date,
            amount_base: amount,
        });
    }
    Ok(flows)
}

/// TWR for one security using its own purchases, sales, and dividends.
// Keep independent period, data-source, and option arguments explicit.
#[allow(clippy::too_many_arguments)]
pub fn position_twr_between(
    transactions: &[Transaction],
    security_id: &str,
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Decimal> {
    let own = transactions_for_security(transactions, security_id);
    let flows = position_flows(&own, base, rates)?;

    let mut breaks: BTreeSet<NaiveDate> = BTreeSet::new();
    breaks.insert(from);
    breaks.insert(to);
    for flow in &flows {
        let boundary = flow.date - Duration::days(1);
        if boundary > from && boundary < to {
            breaks.insert(boundary);
        }
    }
    let breaks: Vec<NaiveDate> = breaks.into_iter().collect();

    let mut points = Vec::with_capacity(breaks.len());
    for (i, date) in breaks.iter().enumerate() {
        let holdings = holdings_at(&own, base, *date, rates, options)?;
        let value = position_value(&holdings, security_id, base, *date, prices, rates)?;
        let external_flow = if i == 0 {
            Decimal::ZERO
        } else {
            let (prev, cur) = (breaks[i - 1], *date);
            flows
                .iter()
                .filter(|f| f.date > prev && f.date <= cur)
                .map(|f| f.amount_base)
                .sum()
        };
        points.push(TwrPoint {
            date: *date,
            end_value: value,
            external_flow,
        });
    }
    time_weighted_return(&points)
}

/// Business-day returns of one position over `(from, to]`, split at its own flows exactly as
/// [`time_weighted_return`] splits them, so the chained returns are the position's TWR. A weekend
/// flow is carried to the next business day, as [`ValueSeries::business_days`] does.
// Keep independent period, data-source, and option arguments explicit.
#[allow(clippy::too_many_arguments)]
fn position_daily_returns(
    own: &[Transaction],
    security_id: &str,
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Vec<(NaiveDate, f64)>> {
    let flows = position_flows(own, base, rates)?;
    let events = ordered_events(own, options.corporate_actions);
    let mut builder = HoldingsBuilder::new(own, base, rates, options);
    let mut next_event = 0;
    let mut apply_through = |day: NaiveDate, builder: &mut HoldingsBuilder<'_>| -> Result<()> {
        while next_event < events.len() && events[next_event].date() <= day {
            builder.apply(events[next_event])?;
            next_event += 1;
        }
        Ok(())
    };

    apply_through(from, &mut builder)?;
    let mut previous = position_value(builder.holdings(), security_id, base, from, prices, rates)?;
    let mut carried = Decimal::ZERO;
    let mut out = Vec::new();
    let mut day = from + Duration::days(1);
    while day <= to {
        apply_through(day, &mut builder)?;
        carried += flows
            .iter()
            .filter(|f| f.date == day)
            .map(|f| f.amount_base)
            .sum::<Decimal>();
        if matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            day += Duration::days(1);
            continue;
        }
        let value = position_value(builder.holdings(), security_id, base, day, prices, rates)?;
        let start = previous + carried;
        let factor = if value.is_zero() && carried.is_sign_negative() && !previous.is_zero() {
            // Sold off: what came out over what was there, as `time_weighted_return` reads it.
            Some(-carried / previous)
        } else if start.is_zero() {
            // Not held on either side of the day: no return to speak of.
            None
        } else {
            Some(value / start)
        };
        // `f64` because a return is a statistic here, the input of a standard deviation.
        if let Some(r) = factor.and_then(|f| (f - Decimal::ONE).to_f64()) {
            out.push((day, r));
        }
        previous = value;
        carried = Decimal::ZERO;
        day += Duration::days(1);
    }
    Ok(out)
}

/// XIRR for one security at `as_of`, using investor cash-flow signs.
pub fn position_xirr(
    transactions: &[Transaction],
    security_id: &str,
    base: &str,
    as_of: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Decimal> {
    let own = transactions_for_security(transactions, security_id);
    let holdings = holdings_at(&own, base, as_of, rates, options)?;
    let value = position_value(&holdings, security_id, base, as_of, prices, rates)?;

    let mut flows: Vec<CashFlow> = position_flows(&own, base, rates)?
        .into_iter()
        .filter(|f| f.date <= as_of)
        .map(|f| CashFlow {
            date: f.date,
            amount_base: -f.amount_base,
        })
        .collect();
    flows.push(CashFlow {
        date: as_of,
        amount_base: value,
    });
    xirr(&flows)
}

/// Return and contribution for one security; unsolvable TWR/XIRR values are `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionReturnRow {
    pub security_id: String,
    /// Return fraction, computed from the security's own flows.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr: Option<Decimal>,
    /// The same return restated as a yearly rate; `None` when the TWR itself is.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub twr_annualized: Option<Decimal>,
    /// IRR over the security's full history through `to`, not just this period.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub xirr: Option<Decimal>,
    /// Base-currency P/L: value change minus net position flows.
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl_base: Decimal,
    /// `pnl_base` against the capital this position had at work; `None` when it had none.
    /// Unlike TWR this one is not chained, so it answers "what did the money in it earn".
    #[serde(with = "rust_decimal::serde::str_option")]
    pub absolute_performance: Option<Decimal>,
    /// Fees and taxes paid on this instrument in the period, trade commissions included.
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    /// Contribution to portfolio return as a fraction.
    #[serde(with = "rust_decimal::serde::str")]
    pub contribution: Decimal,
    /// The position's own daily-return risk over the period, from the same business-day returns
    /// its TWR chains; `None` with fewer than two such days.
    pub risk: Option<PositionRisk>,
}

/// Per-position risk statistics. `f64` for the reason given on [`crate::calc::RiskMetrics`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRisk {
    pub volatility: f64,
    pub semi_deviation: f64,
    /// Depth of the deepest drawdown as a fraction, `0.0` when the position never fell.
    pub max_drawdown: f64,
    /// Calendar days the deepest drawdown lasted, to the period end while still underwater.
    pub max_drawdown_days: Option<i64>,
}

/// Computes all position returns with shared market data; contribution is `pnl_base` over the
/// capital the *portfolio* had at work, so contributions add up to the portfolio's own return —
/// see [`super::dietz_capital`]. Add up to it, not necessarily to it exactly: account interest
/// belongs to no instrument, so a portfolio earning any leaves the column short by that much.
/// `absolute_performance` divides by the position's own capital instead, which is why the two
/// columns disagree and should.
#[allow(clippy::too_many_arguments)]
pub fn position_returns(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Vec<PositionReturnRow>> {
    let start_holdings = holdings_at(transactions, base, from, rates, options)?;
    let end_holdings = holdings_at(transactions, base, to, rates, options)?;
    let start_value = value_holdings(&start_holdings, base, from, prices, rates)?.total_value_base;
    let capital = dietz_capital(
        start_value,
        end_holdings
            .external_flows
            .iter()
            .map(|f| (f.date, f.amount_base)),
        from,
        to,
    );
    let costs = costs_paid_by_security(transactions, base, from, to, rates)?;

    // Include positions open at `from` or active during the period.
    let mut ids: BTreeSet<String> = start_holdings
        .positions
        .iter()
        .filter(|(_, p)| !p.is_closed())
        .map(|(id, _)| id.clone())
        .collect();
    for t in transactions.iter().filter(|t| t.date > from && t.date <= to) {
        if let Some(id) = &t.security_id {
            ids.insert(id.clone());
        }
    }

    let mut rows = Vec::with_capacity(ids.len());
    for security_id in ids {
        let own = transactions_for_security(transactions, &security_id);
        // Portfolio holdings give the same position value as security-only holdings.
        let start = position_value(&start_holdings, &security_id, base, from, prices, rates)?;
        let end = position_value(&end_holdings, &security_id, base, to, prices, rates)?;
        let flows = position_flows(&own, base, rates)?;
        let net_flow: Decimal = flows
            .iter()
            .filter(|f| f.date > from && f.date <= to)
            .map(|f| f.amount_base)
            .sum();
        // P/L is end - start - net flow; a dividend flow is negative and adds income.
        let pnl_base = end - start - net_flow;
        // The position's own denominator: what the portfolio held in *this* instrument.
        let own_capital = dietz_capital(start, flows.iter().map(|f| (f.date, f.amount_base)), from, to);
        let cost = costs.get(&security_id).cloned().unwrap_or_default();
        let twr = unsolvable_to_none(position_twr_between(
            &own,
            &security_id,
            base,
            from,
            to,
            prices,
            rates,
            options,
        ))?;

        let returns = position_daily_returns(&own, &security_id, base, from, to, prices, rates, options)?;
        let risk = (returns.len() >= 2).then(|| {
            // The risk-free rate only moves Sharpe, which this row does not carry.
            let metrics = metrics_from_returns(&returns, 0.0);
            PositionRisk {
                volatility: metrics.volatility,
                semi_deviation: metrics.semi_deviation,
                max_drawdown: metrics.max_drawdown.as_ref().map_or(0.0, |dd| dd.depth),
                max_drawdown_days: metrics.max_drawdown_days,
            }
        });

        rows.push(PositionReturnRow {
            security_id: security_id.clone(),
            twr,
            twr_annualized: twr.and_then(|r| annualize(r, from, to)),
            xirr: unsolvable_to_none(position_xirr(
                &own,
                &security_id,
                base,
                to,
                prices,
                rates,
                options,
            ))?,
            pnl_base,
            absolute_performance: (own_capital > Decimal::ZERO).then(|| pnl_base / own_capital),
            fees_base: cost.fees_base,
            taxes_base: cost.taxes_base,
            contribution: if capital <= Decimal::ZERO {
                Decimal::ZERO
            } else {
                pnl_base / capital
            },
            risk,
        });
    }
    Ok(rows)
}

/// Converts an unsolvable math error to an empty table cell; data errors still propagate.
fn unsolvable_to_none(result: Result<Decimal>) -> Result<Option<Decimal>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(Error::Math(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Converts portfolio flows to investor signs for XIRR: deposits negative,
/// current value positive.
pub(super) fn investor_cash_flows(holdings: &Holdings, valuation: &PortfolioValuation) -> Vec<CashFlow> {
    let mut flows: Vec<CashFlow> = holdings
        .external_flows
        .iter()
        .map(|f| CashFlow {
            date: f.date,
            amount_base: -f.amount_base,
        })
        .collect();
    flows.push(CashFlow {
        date: valuation.date,
        amount_base: valuation.total_value_base,
    });
    flows
}

impl PortfolioAnalytics<'_> {
    pub fn twr(&self, from: NaiveDate, to: NaiveDate) -> Result<Decimal> {
        let tx = self.transactions_until(Some(to))?;
        twr_between_with(
            &tx,
            self.base_currency(),
            from,
            to,
            self.store,
            self.store,
            self.options(),
        )
    }

    /// Money-weighted return at `as_of`.
    pub fn xirr(&self, as_of: NaiveDate) -> Result<Decimal> {
        let holdings = self.holdings_at(as_of)?;
        let valuation = value_holdings(&holdings, self.base_currency(), as_of, self.store, self.store)?;
        xirr(&investor_cash_flows(&holdings, &valuation))
    }

    /// Daily value series for a date range.
    pub fn series(&self, range: DateRange) -> Result<ValueSeries> {
        let transactions = self.transactions_until(Some(range.to))?;
        let (prices, rates) = self.market_data(range.to)?;
        value_series(
            &transactions,
            self.base_currency(),
            range,
            &prices,
            &rates,
            self.options(),
        )
    }

    /// The money view of a range, alongside the return view [`Self::twr`] gives.
    pub fn period_summary(&self, range: DateRange) -> Result<PeriodSummary> {
        Ok(period_summary(&self.series(range)?))
    }

    /// The calculation sheet of a range: one row per calendar chunk, showing how the opening
    /// value, the flows and what was earned add up to the closing one.
    pub fn calculation_sheet(&self, range: DateRange, period: Period) -> Result<CalculationSheet> {
        let transactions = self.transactions_until(Some(range.to))?;
        let (prices, rates) = self.market_data(range.to)?;
        let series = value_series(
            &transactions,
            self.base_currency(),
            range,
            &prices,
            &rates,
            self.options(),
        )?;
        let holdings = build_holdings_with(&transactions, self.base_currency(), &rates, self.options())?;
        calculation_sheet(&series, &holdings, &transactions, &rates, period)
    }

    /// Fees and taxes paid over a range, trade commissions included — the numerator of a
    /// fee or tax rate. See [`super::costs_paid`] for why this is not the charges report.
    pub fn costs(&self, from: NaiveDate, to: NaiveDate) -> Result<ChargeSummary> {
        let transactions = self.transactions_until(Some(to))?;
        costs_paid(&transactions, self.base_currency(), from, to, self.store)
    }

    /// TWR for one security over a period.
    pub fn position_twr(&self, security_id: &str, from: NaiveDate, to: NaiveDate) -> Result<Decimal> {
        let transactions = self.transactions_until(Some(to))?;
        let (prices, rates) = self.market_data(to)?;
        position_twr_between(
            &transactions,
            security_id,
            self.base_currency(),
            from,
            to,
            &prices,
            &rates,
            self.options(),
        )
    }

    /// IRR for one security at a date.
    pub fn position_xirr(&self, security_id: &str, as_of: NaiveDate) -> Result<Decimal> {
        let transactions = self.transactions_until(Some(as_of))?;
        let (prices, rates) = self.market_data(as_of)?;
        position_xirr(
            &transactions,
            security_id,
            self.base_currency(),
            as_of,
            &prices,
            &rates,
            self.options(),
        )
    }

    /// Returns and contributions for all positions using one shared history pass.
    pub fn position_returns(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<PositionReturnRow>> {
        let transactions = self.transactions_until(Some(to))?;
        let (prices, rates) = self.market_data(to)?;
        position_returns(
            &transactions,
            self.base_currency(),
            from,
            to,
            &prices,
            &rates,
            self.options(),
        )
    }
}
