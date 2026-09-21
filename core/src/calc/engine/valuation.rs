//! What is held on a date, and what it is worth there.
//!
//! The order is always transactions to `Holdings` to `PortfolioValuation`: holdings are
//! deterministic and price-free, and only the valuation reaches for market data.

use crate::calc::cash_balances;
use crate::calc::{
    CostBasisRow, Holdings, HoldingsOptions, PortfolioValuation, build_holdings_with, compare_cost_basis,
    value_holdings,
};
use crate::error::Result;
use crate::fx::{RateCache, RateLookup};
use crate::market::{PriceCache, PriceLookup};
use crate::model::{CorporateAction, CostBasisMethod, Transaction};
use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

use super::PortfolioAnalytics;

/// Values a portfolio at a date from raw transactions; date filtering stays outside storage.
pub fn valuation_at(
    transactions: &[Transaction],
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<PortfolioValuation> {
    valuation_at_with(
        transactions,
        base,
        date,
        prices,
        rates,
        HoldingsOptions::default(),
    )
}

/// Values a portfolio with explicit holdings options.
pub fn valuation_at_with(
    transactions: &[Transaction],
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<PortfolioValuation> {
    let holdings = holdings_at(transactions, base, date, rates, options)?;
    value_holdings(&holdings, base, date, prices, rates)
}

/// Builds holdings through `date`, including only corporate actions through that date.
pub fn holdings_at(
    transactions: &[Transaction],
    base: &str,
    date: NaiveDate,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Holdings> {
    let upto: Vec<Transaction> = transactions.iter().filter(|t| t.date <= date).cloned().collect();
    let actions: Vec<CorporateAction> = options
        .corporate_actions
        .iter()
        .filter(|a| a.date <= date)
        .cloned()
        .collect();
    build_holdings_with(&upto, base, rates, options.with_corporate_actions(&actions))
}

impl PortfolioAnalytics<'_> {
    /// Cash balances for selected accounts, computed from raw transactions.
    pub fn cash_balances(&self, date: NaiveDate) -> Result<BTreeMap<String, BTreeMap<Currency, Decimal>>> {
        let all = self
            .store
            .transactions_for_accounts(&self.portfolio.account_ids, Some(date))?;
        let mut balances = cash_balances(&all, &self.accounts, date);
        balances.retain(|id, _| self.scope.contains(id));
        Ok(balances)
    }

    pub fn holdings_at(&self, date: NaiveDate) -> Result<Holdings> {
        let tx = self.transactions_until(Some(date))?;
        holdings_at(&tx, self.base_currency(), date, self.store, self.options())
    }

    pub fn valuation_at(&self, date: NaiveDate) -> Result<PortfolioValuation> {
        let holdings = self.holdings_at(date)?;
        value_holdings(&holdings, self.base_currency(), date, self.store, self.store)
    }

    /// Every open position's purchase value under both cost-basis methods.
    ///
    /// Two holdings passes and one valuation: the methods disagree about the cost of what is
    /// left, never about how much of it there is, so the prices are read once.
    pub fn cost_basis_comparison(&self, date: NaiveDate) -> Result<Vec<CostBasisRow>> {
        let transactions = self.transactions_until(Some(date))?;
        let base = self.base_currency();
        let pass = |method: CostBasisMethod| {
            holdings_at(
                &transactions,
                base,
                date,
                self.store,
                self.options().with_cost_basis(method),
            )
        };
        let fifo = pass(CostBasisMethod::Fifo)?;
        let average = pass(CostBasisMethod::AverageCost)?;
        let valuation = value_holdings(&fifo, base, date, self.store, self.store)?;
        Ok(compare_cost_basis(&fifo, &average, &valuation))
    }

    /// Preloads price and FX series through `upto` for day-by-day analytics.
    pub fn market_data(&self, upto: NaiveDate) -> Result<(PriceCache, RateCache)> {
        let transactions = self.transactions_until(Some(upto))?;
        let security_ids: Vec<String> = transactions
            .iter()
            .filter_map(|t| t.security_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let prices = self.store.price_cache(&security_ids, upto)?;

        let base = self.base_currency().to_string();
        let mut currencies: BTreeSet<String> = transactions.iter().map(|t| t.currency.clone()).collect();
        for id in &security_ids {
            currencies.insert(self.store.get_security(id)?.currency);
        }
        let pairs: Vec<(String, String)> = currencies
            .into_iter()
            .filter(|c| *c != base)
            .map(|c| (c, base.clone()))
            .collect();
        let rates = self.store.rate_cache(&pairs, upto)?;
        Ok((prices, rates))
    }
}
