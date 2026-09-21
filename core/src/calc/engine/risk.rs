//! Volatility, drawdown and the comparison against a benchmark.
//!
//! Risk runs on business days — the annualisation assumes trading days — while the series it
//! reads covers every calendar day.

use crate::calc::{
    BenchmarkComparison, GrowthSeries, Period, PeriodReturn, RiskMetrics, RiskReport, benchmark_return,
    benchmark_series, benchmark_start, compare_to_benchmark, returns_by_period, risk_metrics, risk_report,
};
use crate::calc::{Peak, all_time_high};
use crate::error::{Error, Result};
use crate::market::DateRange;
use chrono::NaiveDate;

use super::PortfolioAnalytics;

impl PortfolioAnalytics<'_> {
    /// Risk metrics for a range; `risk_free_rate` is annual.
    pub fn risk(&self, range: DateRange, risk_free_rate: f64) -> Result<RiskMetrics> {
        Ok(risk_metrics(&self.series(range)?, risk_free_rate))
    }

    /// Risk metrics and chart series in one pass.
    pub fn risk_report(
        &self,
        range: DateRange,
        risk_free_rate: f64,
        window_days: usize,
    ) -> Result<RiskReport> {
        Ok(risk_report(&self.series(range)?, risk_free_rate, window_days))
    }

    /// Return by calendar period, such as month or year.
    pub fn returns_by_period(&self, range: DateRange, period: Period) -> Result<Vec<PeriodReturn>> {
        Ok(returns_by_period(&self.series(range)?, period))
    }

    /// Compares the portfolio with a benchmark security.
    pub fn benchmark(
        &self,
        benchmark_security_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<BenchmarkComparison> {
        let security = self.store.get_security(benchmark_security_id)?;
        // Benchmarks are usually outside the portfolio, so load them separately.
        let prices = self.store.price_cache(std::slice::from_ref(&security.id), to)?;
        let rates = self.store.rate_cache(
            &[(security.currency.clone(), self.base_currency().to_string())],
            to,
        )?;
        // A benchmark younger than the period is compared over what the two have in common:
        // three years of portfolio against one year of index would forge the excess. `from`
        // therefore reports the window that was actually compared, not the one asked for.
        let dates: Vec<NaiveDate> = from.iter_days().take_while(|date| *date <= to).collect();
        let from = benchmark_start(&security.id, &dates, &prices)?.ok_or(Error::MissingMarketData {
            kind: "price",
            key: security.id.clone(),
            date: to,
        })?;
        let benchmark = benchmark_return(&security.id, self.base_currency(), from, to, &prices, &rates)?;
        Ok(compare_to_benchmark(from, to, self.twr(from, to)?, benchmark))
    }

    /// Benchmark growth on the caller-supplied portfolio date grid.
    pub fn benchmark_growth(&self, benchmark_security_id: &str, dates: &[NaiveDate]) -> Result<GrowthSeries> {
        let Some(&last) = dates.last() else {
            return Ok(GrowthSeries::default());
        };
        let security = self.store.get_security(benchmark_security_id)?;
        let prices = self.store.price_cache(std::slice::from_ref(&security.id), last)?;
        let rates = self.store.rate_cache(
            &[(security.currency.clone(), self.base_currency().to_string())],
            last,
        )?;
        benchmark_series(&security.id, self.base_currency(), dates, &prices, &rates)
    }

    /// Highest portfolio value inside a range, and how far below it the range ends.
    pub fn all_time_high(&self, range: DateRange) -> Result<Option<Peak>> {
        Ok(all_time_high(&self.series(range)?))
    }
}
