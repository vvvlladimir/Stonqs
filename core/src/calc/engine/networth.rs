//! Net worth: the portfolio's own total plus the things owned and owed beside it. Read whole,
//! never through the account picker — an asset is not an account, so no lens narrows it
//! (ADR-0092).

use crate::calc::{AfterTax, NetWorth, NetWorthSeries, after_tax, net_worth, net_worth_series};
use crate::error::Result;
use crate::market::DateRange;
use crate::model::{Asset, AssetValue};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::PortfolioAnalytics;

impl PortfolioAnalytics<'_> {
    pub fn net_worth(&self, date: NaiveDate) -> Result<NetWorth> {
        Ok(self.net_worth_with_tax(date, None)?.0)
    }

    /// The reading and, given a rate, what selling the portfolio today would cost in tax
    /// (ADR-0093). Both come off one valuation pass, so the two can never disagree about what is
    /// invested.
    pub fn net_worth_with_tax(
        &self,
        date: NaiveDate,
        tax_rate: Option<Decimal>,
    ) -> Result<(NetWorth, Option<AfterTax>)> {
        let (assets, values) = self.assets()?;
        let valuation = self.whole()?.valuation_at(date)?;
        let reading = net_worth(
            valuation.total_value_base,
            &assets,
            &values,
            self.base_currency(),
            date,
            self.store,
        )?;
        let view = tax_rate
            .map(|rate| {
                after_tax(
                    reading.net_base,
                    valuation.unrealized_pnl_base,
                    reading.owned_base,
                    rate,
                )
            })
            .transpose()?;
        Ok((reading, view))
    }

    /// The net-worth line. The portfolio side comes from the same daily series the performance
    /// screen draws, so the two cannot disagree about what the portfolio was worth.
    pub fn net_worth_series(&self, from: NaiveDate, to: NaiveDate) -> Result<NetWorthSeries> {
        let (assets, values) = self.assets()?;
        let whole = self.whole()?;
        let portfolio = whole.series(DateRange::new(from, to))?;
        net_worth_series(
            &portfolio,
            &assets,
            &values,
            self.base_currency(),
            from,
            to,
            self.store,
        )
    }

    fn assets(&self) -> Result<(Vec<Asset>, Vec<AssetValue>)> {
        Ok((
            self.store.list_assets(&self.portfolio.id)?,
            self.store.list_asset_values(&self.portfolio.id)?,
        ))
    }

    /// The portfolio as a whole, whatever this instance is scoped to.
    fn whole(&self) -> Result<PortfolioAnalytics<'_>> {
        PortfolioAnalytics::new(self.store, self.portfolio)
    }
}
