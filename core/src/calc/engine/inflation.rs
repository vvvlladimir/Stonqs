use super::PortfolioAnalytics;
use super::returns::investor_cash_flows;
use crate::calc::value_holdings;
use crate::calc::{GrowthSeries, RealReturn, inflation_series, real_period_return, real_xirr};
use crate::error::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A period's returns restated in the money of its first day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealPerformance {
    /// The time-weighted return, and the inflation window it was deflated over.
    pub twr: RealReturn,
    /// The money-weighted return with every flow taken at its own date's price level;
    /// `None` for the same reason the nominal one is — too few flows to solve.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub xirr: Option<Decimal>,
}

impl PortfolioAnalytics<'_> {
    /// The consumer-price region this portfolio reports against, if its owner named one.
    pub fn inflation_region(&self) -> Option<&str> {
        self.portfolio.inflation_region.as_deref()
    }

    /// The period's returns with inflation taken out.
    ///
    /// `None` when the portfolio names no region: inflation is a lens the owner turns on, not a
    /// component of value, so its absence is a setting rather than missing market data. A region
    /// that *is* named but has no index stored yet is an error, like any other missing series.
    pub fn real_performance(&self, from: NaiveDate, to: NaiveDate) -> Result<Option<RealPerformance>> {
        let Some(region) = self.inflation_region() else {
            return Ok(None);
        };
        let nominal = self.twr(from, to)?;
        let twr = real_period_return(region, from, to, nominal, self.store)?;

        // The money-weighted return is deflated over the same window the time-weighted one
        // reports, so the two figures on a screen never answer different questions.
        let holdings = self.holdings_at(twr.to)?;
        let valuation = value_holdings(&holdings, self.base_currency(), twr.to, self.store, self.store)?;
        let flows = investor_cash_flows(&holdings, &valuation);
        let xirr = match real_xirr(region, from, &flows, self.store) {
            Ok(v) => Some(v),
            Err(crate::error::Error::Math(_)) => None,
            Err(e) => return Err(e),
        };

        Ok(Some(RealPerformance { twr, xirr }))
    }

    /// What one unit of money costs across the portfolio's date grid, based at the first day —
    /// the same shape [`Self::benchmark_growth`] returns, so a chart draws it as one more line.
    pub fn inflation_growth(&self, dates: &[NaiveDate]) -> Result<Option<GrowthSeries>> {
        let Some(region) = self.inflation_region() else {
            return Ok(None);
        };
        Ok(Some(inflation_series(region, dates, self.store)?))
    }
}
