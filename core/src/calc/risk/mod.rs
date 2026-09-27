use super::{StatSeries, ValueSeries};
use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// Trading days per year; annualization uses `sqrt(252)`.
pub const TRADING_DAYS_PER_YEAR: f64 = 252.0;

/// Drawdown peak, trough, recovery date, and depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drawdown {
    pub peak: NaiveDate,
    pub trough: NaiveDate,
    /// Date the prior peak was recovered; `None` means still underwater.
    pub recovered: Option<NaiveDate>,
    /// Depth as a fraction; `-0.35` means -35%.
    pub depth: f64,
}

impl Drawdown {
    /// Calendar days from the peak to the recovery, or to `as_of` while still underwater.
    /// An open episode measured to its trough would report a hole as already survived.
    pub fn duration_days(&self, as_of: NaiveDate) -> i64 {
        (self.recovered.unwrap_or(as_of) - self.peak).num_days()
    }
}

/// Portfolio risk metrics. `f64` is appropriate because these are statistical ratios,
/// including square roots and annualized powers, not monetary amounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMetrics {
    /// Number of daily returns included.
    pub days: usize,
    /// Annualized daily-return standard deviation times `sqrt(252)`.
    pub volatility: f64,
    /// Annualized downside deviation: the losing days' sizes, over every day there was.
    pub semi_deviation: f64,
    /// Maximum drawdown; `None` when fewer than two returns exist.
    pub max_drawdown: Option<Drawdown>,
    /// Calendar days the deepest episode lasted, counted to the series end while underwater.
    pub max_drawdown_days: Option<i64>,
    /// Longest episode by duration, which is rarely the deepest one: a shallow hole that
    /// never fills is the one that costs years.
    pub longest_drawdown: Option<Drawdown>,
    pub longest_drawdown_days: Option<i64>,
    /// Distance from the running peak at the last point, not the open episode's trough.
    pub current_drawdown: f64,
    /// The peak not yet climbed back to; `None` when the period ends at one.
    pub current_drawdown_since: Option<NaiveDate>,
    /// Annualized `(return - risk-free rate) / volatility`; `None` at zero volatility.
    pub sharpe: Option<f64>,
    /// Annualized geometric return over the period.
    pub annualized_return: f64,
    pub best_day: Option<(NaiveDate, f64)>,
    pub worst_day: Option<(NaiveDate, f64)>,
    /// Share of days with positive returns.
    pub positive_days_share: f64,
}

/// Computes risk metrics from business-day returns and an annual risk-free rate.
pub fn risk_metrics(series: &ValueSeries, risk_free_rate: f64) -> RiskMetrics {
    metrics_from_returns(&return_series(series), risk_free_rate)
}

/// Computes the same metrics from an already prepared return series.
pub(crate) fn metrics_from_returns(returns: &[(NaiveDate, f64)], risk_free_rate: f64) -> RiskMetrics {
    let days = returns.len();
    let values: Vec<f64> = returns.iter().map(|(_, r)| *r).collect();

    let mean = if days == 0 {
        0.0
    } else {
        values.iter().sum::<f64>() / days as f64
    };
    // Use sample variance (n-1): observed returns are a sample of possible days.
    let variance = if days > 1 {
        values.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (days - 1) as f64
    } else {
        0.0
    };
    let volatility = variance.sqrt() * TRADING_DAYS_PER_YEAR.sqrt();

    // Measured from zero and divided by every day, like volatility, so the two are comparable.
    let downside: f64 = values.iter().filter(|r| **r < 0.0).map(|r| r.powi(2)).sum();
    // A portfolio that never fell is asked for explicitly: Rust sums an empty iterator to
    // `-0.0`, which survives the division and the square root and reaches a screen as "-0.00%".
    let semi_deviation = if days > 1 && downside > 0.0 {
        (downside / (days - 1) as f64).sqrt() * TRADING_DAYS_PER_YEAR.sqrt()
    } else {
        0.0
    };

    let cumulative = values.iter().fold(1.0, |acc, r| acc * (1.0 + r));
    let annualized_return = if days == 0 {
        0.0
    } else {
        cumulative.powf(TRADING_DAYS_PER_YEAR / days as f64) - 1.0
    };
    let sharpe = if volatility > 0.0 {
        Some((annualized_return - risk_free_rate) / volatility)
    } else {
        None
    };

    let best = returns.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1));
    let worst = returns.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1));
    let positive_days_share = if days == 0 {
        0.0
    } else {
        values.iter().filter(|r| **r > 0.0).count() as f64 / days as f64
    };

    let episodes = drawdowns(returns);
    let as_of = returns.last().map(|(date, _)| *date);
    // Duration needs the series end, so it is resolved here rather than on `Drawdown` itself.
    let span = |dd: &Drawdown| as_of.map(|end| dd.duration_days(end));
    let longest = as_of.and_then(|end| episodes.iter().max_by_key(|dd| dd.duration_days(end)).cloned());

    RiskMetrics {
        days,
        volatility,
        semi_deviation,
        max_drawdown_days: episodes.first().and_then(span),
        longest_drawdown_days: longest.as_ref().and_then(span),
        longest_drawdown: longest,
        current_drawdown: underwater(returns),
        current_drawdown_since: episodes
            .iter()
            .find(|dd| dd.recovered.is_none())
            .map(|dd| dd.peak),
        max_drawdown: episodes.into_iter().next(),
        sharpe,
        annualized_return,
        best_day: best,
        worst_day: worst,
        positive_days_share,
    }
}

/// Risk-screen metrics and chart series computed from one value series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskReport {
    pub metrics: RiskMetrics,
    /// Rolling-volatility window in trading days.
    pub window_days: usize,
    pub returns: StatSeries,
    pub rolling_volatility: StatSeries,
    /// Daily drawdown from the previous peak.
    pub drawdown: StatSeries,
    /// Drawdown episodes, deepest first.
    pub episodes: Vec<Drawdown>,
}

/// Risk metrics and chart series for one value series.
pub fn risk_report(series: &ValueSeries, risk_free_rate: f64, window_days: usize) -> RiskReport {
    let returns = return_series(series);
    let mut stat = StatSeries::default();
    for (date, r) in &returns {
        stat.push(*date, *r);
    }
    RiskReport {
        metrics: metrics_from_returns(&returns, risk_free_rate),
        window_days,
        rolling_volatility: rolling_volatility(&returns, window_days),
        drawdown: drawdown_series(&returns),
        episodes: drawdowns(&returns),
        returns: stat,
    }
}

/// Converts the business-day value series to the shared `f64` return series.
pub fn return_series(series: &ValueSeries) -> Vec<(NaiveDate, f64)> {
    series
        .business_days()
        .daily_returns()
        .into_iter()
        .map(|(d, r)| (d, r.to_f64().unwrap_or(0.0)))
        .collect()
}

/// Rolling annualized volatility over a trading-day window.
/// The initial `window - 1` points are omitted because no window exists yet.
pub fn rolling_volatility(returns: &[(NaiveDate, f64)], window: usize) -> StatSeries {
    let mut out = StatSeries::default();
    // A one-day window has no sample variance.
    if window < 2 || returns.len() < window {
        return out;
    }
    for end in window..=returns.len() {
        let slice = &returns[end - window..end];
        let mean = slice.iter().map(|(_, r)| *r).sum::<f64>() / window as f64;
        let variance = slice.iter().map(|(_, r)| (r - mean).powi(2)).sum::<f64>() / (window - 1) as f64;
        out.push(
            slice[window - 1].0,
            variance.sqrt() * TRADING_DAYS_PER_YEAR.sqrt(),
        );
    }
    out
}

/// Distance from the running peak at the end of the series, on the same chained-return index.
fn underwater(returns: &[(NaiveDate, f64)]) -> f64 {
    let (index, peak) = returns.iter().fold((1.0_f64, 1.0_f64), |(index, peak), (_, r)| {
        let index = index * (1.0 + r);
        (index, peak.max(index))
    });
    index / peak - 1.0
}

/// Daily drawdown series from the same chained-return index used by [`drawdowns`].
pub fn drawdown_series(returns: &[(NaiveDate, f64)]) -> StatSeries {
    let mut out = StatSeries::default();
    let mut index: f64 = 1.0;
    let mut peak: f64 = 1.0;
    for (date, r) in returns {
        index *= 1.0 + r;
        peak = peak.max(index);
        out.push(*date, index / peak - 1.0);
    }
    out
}

/// Returns every drawdown, deepest first, using flow-adjusted returns so deposits
/// cannot hide investment losses; an open episode has `recovered = None`.
pub fn drawdowns(returns: &[(NaiveDate, f64)]) -> Vec<Drawdown> {
    let mut out: Vec<Drawdown> = Vec::new();
    let Some((first_date, _)) = returns.first() else {
        return out;
    };

    let mut index = 1.0;
    let mut peak = 1.0;
    let mut peak_date = *first_date;
    let mut current: Option<Drawdown> = None;

    for (date, r) in returns {
        index *= 1.0 + r;
        if index >= peak {
            peak = index;
            peak_date = *date;
            // Reaching a new peak closes the current episode on this date.
            if let Some(mut episode) = current.take() {
                episode.recovered = Some(*date);
                out.push(episode);
            }
            continue;
        }
        let depth = index / peak - 1.0;
        match current.as_mut() {
            // Keep the deepest trough while the episode continues.
            Some(episode) if depth < episode.depth => {
                episode.trough = *date;
                episode.depth = depth;
            }
            Some(_) => {}
            None => {
                current = Some(Drawdown {
                    peak: peak_date,
                    trough: *date,
                    recovered: None,
                    depth,
                })
            }
        }
    }
    out.extend(current);
    out.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    out
}

#[cfg(test)]
mod tests;
