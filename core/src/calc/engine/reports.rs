//! The ledger read back: realised gains, income, dividends, payments and round trips.

use crate::calc::{
    DividendProfile, DividendSummary, PaymentGrid, PaymentPeriod, TradeBook, TradingVolume, closed_trades,
    dividend_profiles, open_trades, payment_grid, trading_volume,
};
use crate::calc::{
    IncomeRecord, RealizedSummary, TaxonomyIncome, capital_gains_by_year, dividends_by_year, income_between,
    income_by_taxonomy, income_of_kind, value_holdings,
};
use crate::error::Result;
use crate::model::TransactionKind;
use chrono::NaiveDate;
use std::collections::BTreeMap;

use super::PortfolioAnalytics;

impl PortfolioAnalytics<'_> {
    /// Realized gains by year through `as_of`.
    pub fn capital_gains(&self, as_of: NaiveDate) -> Result<BTreeMap<i32, RealizedSummary>> {
        Ok(capital_gains_by_year(&self.holdings_at(as_of)?.realized))
    }

    /// Income events in a period; event dates are used rather than state differences.
    pub fn income(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<IncomeRecord>> {
        Ok(income_between(&self.holdings_at(to)?, from, to))
    }

    /// Income of a period split through a classification tree. The tree classifies payers, so
    /// this needs no valuation and a payer sold since still reports where it paid from.
    /// `kind` narrows here rather than on the caller's side, as every income rollup does.
    pub fn income_by_taxonomy(
        &self,
        taxonomy_id: &str,
        from: NaiveDate,
        to: NaiveDate,
        kind: Option<TransactionKind>,
    ) -> Result<TaxonomyIncome> {
        let income = income_of_kind(&self.income(from, to)?, kind);
        let nodes = self.store.taxonomy_nodes(taxonomy_id)?;
        let assignments = self.taxonomy_assignments(taxonomy_id)?;
        let excluded = self.store.taxonomy_exclusions(taxonomy_id)?;
        Ok(income_by_taxonomy(&income, &nodes, &assignments, &excluded))
    }

    /// Dividends by year through `as_of`.
    pub fn dividends(&self, as_of: NaiveDate) -> Result<BTreeMap<i32, DividendSummary>> {
        Ok(dividends_by_year(&self.holdings_at(as_of)?.income))
    }

    /// Payment schedule and yearly total per paying instrument, over the whole history:
    /// a frequency read off one window would name the window, not the payer.
    pub fn dividend_profiles(&self, as_of: NaiveDate) -> Result<BTreeMap<String, DividendProfile>> {
        Ok(dividend_profiles(&self.holdings_at(as_of)?.income, as_of))
    }

    /// Every dated figure of the period on one axis: income by kind and by payer, costs,
    /// savings and closed-trade results. Built at `to`, then filtered to the window inside
    /// [`payment_grid`] — a rollup of a window still needs the lots the window opened with.
    pub fn payments(&self, from: NaiveDate, to: NaiveDate, period: PaymentPeriod) -> Result<PaymentGrid> {
        Ok(payment_grid(&self.holdings_at(to)?, from, to, period))
    }

    /// Open and closed trades at a date; both come out of the same holdings pass.
    pub fn trades(&self, as_of: NaiveDate) -> Result<TradeBook> {
        let holdings = self.holdings_at(as_of)?;
        let valuation = value_holdings(&holdings, self.base_currency(), as_of, self.store, self.store)?;
        Ok(TradeBook {
            open: open_trades(&holdings, &valuation),
            closed: closed_trades(&holdings.realized),
        })
    }

    /// Bought and sold over a period — the numerator of the turnover rate, whose denominator
    /// is [`PeriodSummary::average_capital_base`].
    pub fn trading_volume(&self, from: NaiveDate, to: NaiveDate) -> Result<TradingVolume> {
        let transactions = self.transactions_until(Some(to))?;
        trading_volume(&transactions, self.base_currency(), from, to, self.store)
    }
}
