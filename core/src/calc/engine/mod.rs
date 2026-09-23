//! `PortfolioAnalytics` glues the rest of `calc` together: it owns the store, the portfolio and
//! the scope, and every reading is a method on it.
//!
//! The methods are grouped by subject, one file each, all writing into the same type — the
//! struct and its lifetime live here, and so does everything the groups share: what the scope
//! is, which transactions it admits, and where the prices and rates come from.

mod allocation;
mod goals;
mod inflation;
mod reports;
mod returns;
mod risk;
mod valuation;

pub use inflation::RealPerformance;
pub use returns::{PositionReturnRow, PositionRisk};
pub use returns::{position_returns, position_twr_between, position_xirr, twr_between, twr_between_with};
pub use valuation::{holdings_at, valuation_at, valuation_at_with};

use super::{HoldingsOptions, scoped_transactions};
use crate::error::Result;
use crate::model::{Account, CorporateAction, Portfolio, Security, Transaction};
use crate::storage::Store;
use chrono::NaiveDate;
use std::collections::BTreeSet;

/// Store-backed facade that gathers data and calls the pure calculation functions.
pub struct PortfolioAnalytics<'a> {
    store: &'a Store,
    portfolio: &'a Portfolio,
    /// Corporate actions cached at construction for repeated calculations.
    corporate_actions: Vec<CorporateAction>,
    /// All portfolio accounts, needed to resolve depot settlement accounts.
    accounts: Vec<Account>,
    /// Selected account IDs; defaults to the whole portfolio.
    scope: Vec<String>,
}

impl<'a> PortfolioAnalytics<'a> {
    pub fn new(store: &'a Store, portfolio: &'a Portfolio) -> Result<Self> {
        let accounts = portfolio
            .account_ids
            .iter()
            .map(|id| store.get_account(id))
            .collect::<Result<Vec<_>>>()?;
        Ok(PortfolioAnalytics {
            store,
            portfolio,
            corporate_actions: store.list_corporate_actions()?,
            scope: portfolio.account_ids.clone(),
            accounts,
        })
    }

    /// Restricts calculations to selected portfolio accounts; linked legs are
    /// rewritten only after reading the full portfolio.
    pub fn scoped_to(mut self, account_ids: &[String]) -> Self {
        self.scope = self
            .portfolio
            .account_ids
            .iter()
            .filter(|id| account_ids.contains(id))
            .cloned()
            .collect();
        self
    }

    /// Accounts included in the current scope.
    pub fn scope_accounts(&self) -> Vec<&Account> {
        self.accounts
            .iter()
            .filter(|a| self.scope.contains(&a.id))
            .collect()
    }

    pub fn base_currency(&self) -> &str {
        &self.portfolio.base_currency
    }

    /// Holdings options assembled from portfolio settings and reference data.
    pub fn options(&self) -> HoldingsOptions<'_> {
        HoldingsOptions::default()
            .with_cost_basis(self.portfolio.cost_basis_method)
            .with_corporate_actions(&self.corporate_actions)
    }

    /// Portfolio transactions rewritten for the selected account scope.
    pub fn transactions_until(&self, date: Option<NaiveDate>) -> Result<Vec<Transaction>> {
        let all = self
            .store
            .transactions_for_accounts(&self.portfolio.account_ids, date)?;
        Ok(scoped_transactions(&all, &self.accounts, &self.scope))
    }

    /// Date of the portfolio's first transaction.
    pub fn inception(&self) -> Result<Option<NaiveDate>> {
        Ok(self.transactions_until(None)?.first().map(|t| t.date))
    }

    /// Securities referenced by portfolio transactions.
    fn securities(&self) -> Result<Vec<Security>> {
        let ids: BTreeSet<String> = self
            .transactions_until(None)?
            .iter()
            .filter_map(|t| t.security_id.clone())
            .collect();
        ids.iter().map(|id| self.store.get_security(id)).collect()
    }
}
