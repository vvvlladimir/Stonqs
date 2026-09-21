use super::{
    Holdings, HoldingsBuilder, HoldingsOptions, TwrPoint, ordered_events, time_weighted_return,
    value_holdings,
};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::market::{DateRange, PriceLookup};
use crate::model::Transaction;
use crate::money::{Currency, normalize_currency};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Daily portfolio values and external flows used by charts and time-based metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueSeries {
    pub base_currency: Currency,
    pub dates: Vec<NaiveDate>,
    /// End-of-day value: securities at that day's price plus cash.
    pub total_value_base: Vec<Decimal>,
    /// External flow on this day; `+` is a deposit.
    pub external_flow_base: Vec<Decimal>,
}

impl ValueSeries {
    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }

    pub fn first_date(&self) -> Option<NaiveDate> {
        self.dates.first().copied()
    }

    pub fn last_date(&self) -> Option<NaiveDate> {
        self.dates.last().copied()
    }

    /// Value on a date present in the series.
    pub fn value_on(&self, date: NaiveDate) -> Option<Decimal> {
        self.dates
            .binary_search(&date)
            .ok()
            .map(|i| self.total_value_base[i])
    }

    /// Converts the series to TWR points; the first day's flow is the baseline and is ignored.
    pub fn twr_points(&self) -> Vec<TwrPoint> {
        self.dates
            .iter()
            .enumerate()
            .map(|(i, date)| TwrPoint {
                date: *date,
                end_value: self.total_value_base[i],
                external_flow: if i == 0 {
                    Decimal::ZERO
                } else {
                    self.external_flow_base[i]
                },
            })
            .collect()
    }

    /// True TWR for the full series.
    pub fn twr(&self) -> Result<Decimal> {
        time_weighted_return(&self.twr_points())
    }

    /// Daily flow-adjusted returns: `r = V_t / (V_{t-1} + F_t) - 1`.
    pub fn daily_returns(&self) -> Vec<(NaiveDate, Decimal)> {
        let mut out = Vec::with_capacity(self.len().saturating_sub(1));
        for i in 1..self.len() {
            let start = self.total_value_base[i - 1] + self.external_flow_base[i];
            if start.is_zero() {
                continue;
            }
            out.push((self.dates[i], self.total_value_base[i] / start - Decimal::ONE));
        }
        out
    }

    /// Cumulative growth from flow-adjusted returns; the final value is `1 + TWR`.
    pub fn growth(&self) -> GrowthSeries {
        let mut out = GrowthSeries::default();
        let Some(first) = self.first_date() else {
            return out;
        };
        out.push(first, Decimal::ONE);
        let mut acc = Decimal::ONE;
        for (date, r) in self.daily_returns() {
            acc *= Decimal::ONE + r;
            out.push(date, acc);
        }
        out
    }

    /// Returns business days only; weekend flows are carried to the next business day.
    pub fn business_days(&self) -> ValueSeries {
        let mut out = ValueSeries {
            base_currency: self.base_currency.clone(),
            dates: Vec::new(),
            total_value_base: Vec::new(),
            external_flow_base: Vec::new(),
        };
        let mut carried = Decimal::ZERO;
        for i in 0..self.len() {
            carried += self.external_flow_base[i];
            if matches!(self.dates[i].weekday(), Weekday::Sat | Weekday::Sun) {
                continue;
            }
            out.dates.push(self.dates[i]);
            out.total_value_base.push(self.total_value_base[i]);
            out.external_flow_base.push(carried);
            carried = Decimal::ZERO;
        }
        out
    }
}

/// Daily statistics used by risk charts; `f64` is sufficient for non-monetary ratios.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatSeries {
    pub dates: Vec<NaiveDate>,
    pub values: Vec<f64>,
}

impl StatSeries {
    pub fn push(&mut self, date: NaiveDate, value: f64) {
        self.dates.push(date);
        self.values.push(value);
    }

    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }
}

/// Growth of one invested unit: `1.0` initially, `1.12` means +12%.
/// Decimal values keep the final point exactly aligned with TWR.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthSeries {
    pub dates: Vec<NaiveDate>,
    pub values: Vec<Decimal>,
}

impl GrowthSeries {
    pub fn push(&mut self, date: NaiveDate, value: Decimal) {
        self.dates.push(date);
        self.values.push(value);
    }

    pub fn len(&self) -> usize {
        self.dates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dates.is_empty()
    }
}

/// Builds daily values by applying events once in chronological order.
/// Long ranges should use preloaded price and FX caches to avoid per-day SQL queries.
pub fn value_series(
    transactions: &[Transaction],
    base: &str,
    range: DateRange,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<ValueSeries> {
    let base = normalize_currency(base);
    let events = ordered_events(transactions, options.corporate_actions);
    let mut builder = HoldingsBuilder::new(transactions, &base, rates, options);

    let mut series = ValueSeries {
        base_currency: base.clone(),
        dates: Vec::new(),
        total_value_base: Vec::new(),
        external_flow_base: Vec::new(),
    };

    let mut next_event = 0;
    let mut counted_flows = 0;
    let mut day = range.from;
    while day <= range.to {
        while next_event < events.len() && events[next_event].date() <= day {
            builder.apply(events[next_event])?;
            next_event += 1;
        }

        let holdings: &Holdings = builder.holdings();
        // Count only new flows on this day; pre-period history is in the initial value.
        let flow: Decimal = holdings.external_flows[counted_flows..]
            .iter()
            .filter(|f| f.date == day)
            .map(|f| f.amount_base)
            .sum();
        counted_flows = holdings.external_flows.len();

        let valuation = value_holdings(holdings, &base, day, prices, rates)?;
        series.dates.push(day);
        series.total_value_base.push(valuation.total_value_base);
        series.external_flow_base.push(flow);

        day += Duration::days(1);
    }

    Ok(series)
}

/// Modified Dietz denominator: the opening value plus every flow weighted by the share of
/// the period it stayed invested. Callers differ only in where their flows come from — a
/// value series carries them per day, a position derives them from its own transactions.
pub(crate) fn dietz_capital<I>(start_value: Decimal, flows: I, from: NaiveDate, to: NaiveDate) -> Decimal
where
    I: IntoIterator<Item = (NaiveDate, Decimal)>,
{
    let days = (to - from).num_days();
    if days <= 0 {
        return start_value;
    }
    let period = Decimal::from(days);
    flows
        .into_iter()
        .filter(|(date, _)| *date > from && *date <= to)
        .fold(start_value, |capital, (date, amount)| {
            capital + amount * Decimal::from((to - date).num_days()) / period
        })
}
