//! Splitting the portfolio through a classification tree, and what it would take to meet a
//! target. Cash is a subject like any other; the maths never branches on "security or account".

use crate::calc::{
    Allocation, MemberScope, NodeMember, RebalanceOptions, RebalancePlan, UNCLASSIFIED_KEY,
    allocation_by_account, allocation_by_currency, allocation_by_security, allocation_by_taxonomy,
    allocation_members, allocation_tree, rebalance, value_holdings,
};
use crate::calc::{Assignment, CashSubject, TaxonomySubject, taxonomy_subjects};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{Account, AllocationTarget};
use chrono::NaiveDate;

use super::PortfolioAnalytics;

impl PortfolioAnalytics<'_> {
    /// Builds taxonomy subjects; cash conversion needs storage, while `calc` stays storage-free.
    pub fn taxonomy_subjects(&self, taxonomy_id: &str, date: NaiveDate) -> Result<Vec<TaxonomySubject>> {
        let valuation = self.valuation_at(date)?;
        let base = self.base_currency().to_string();
        let mut cash: Vec<CashSubject> = Vec::new();
        for (account_id, by_currency) in self.cash_balances(date)? {
            let name = self
                .accounts
                .iter()
                .find(|a| a.id == account_id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| account_id.clone());
            for (currency, amount) in by_currency {
                if amount.is_zero() {
                    continue;
                }
                cash.push(CashSubject {
                    value_base: self.store.convert(amount, &currency, &base, date)?,
                    account_id: account_id.clone(),
                    account_name: name.clone(),
                    currency,
                });
            }
        }
        let excluded = self.store.taxonomy_exclusions(taxonomy_id)?;
        Ok(taxonomy_subjects(
            &valuation,
            &self.securities()?,
            &cash,
            &excluded,
        ))
    }

    /// Returns security and cash assignments in one list.
    pub fn taxonomy_assignments(&self, taxonomy_id: &str) -> Result<Vec<Assignment>> {
        let mut out: Vec<Assignment> = self
            .store
            .classifications_for_taxonomy(taxonomy_id)?
            .iter()
            .map(Assignment::from)
            .collect();
        out.extend(
            self.store
                .cash_classifications_for_taxonomy(taxonomy_id)?
                .iter()
                .map(Assignment::from),
        );
        Ok(out)
    }

    /// Taxonomy allocation at a date.
    pub fn allocation_by_taxonomy(&self, taxonomy_id: &str, date: NaiveDate) -> Result<Allocation> {
        let subjects = self.taxonomy_subjects(taxonomy_id, date)?;
        let nodes = self.store.taxonomy_nodes(taxonomy_id)?;
        let assignments = self.taxonomy_assignments(taxonomy_id)?;
        Ok(allocation_by_taxonomy(&subjects, &nodes, &assignments))
    }

    /// Full taxonomy tree with subject tiles.
    pub fn allocation_tree(&self, taxonomy_id: &str, date: NaiveDate) -> Result<Allocation> {
        let subjects = self.taxonomy_subjects(taxonomy_id, date)?;
        let nodes = self.store.taxonomy_nodes(taxonomy_id)?;
        let assignments = self.taxonomy_assignments(taxonomy_id)?;
        Ok(allocation_tree(&subjects, &nodes, &assignments))
    }

    /// Subjects and assigned fractions for a taxonomy level.
    pub fn allocation_members(
        &self,
        taxonomy_id: &str,
        node_id: Option<&str>,
        date: NaiveDate,
    ) -> Result<Vec<NodeMember>> {
        let subjects = self.taxonomy_subjects(taxonomy_id, date)?;
        let nodes = self.store.taxonomy_nodes(taxonomy_id)?;
        let assignments = self.taxonomy_assignments(taxonomy_id)?;
        // The UI passes the allocation bucket key, including the unclassified key.
        let scope = match node_id {
            None => MemberScope::Portfolio,
            Some(UNCLASSIFIED_KEY) => MemberScope::Unclassified,
            Some(id) => MemberScope::Node(id),
        };
        Ok(allocation_members(&subjects, &nodes, &assignments, scope))
    }

    pub fn allocation_by_currency(&self, date: NaiveDate) -> Result<Allocation> {
        let holdings = self.holdings_at(date)?;
        let valuation = value_holdings(&holdings, self.base_currency(), date, self.store, self.store)?;
        allocation_by_currency(&valuation, &holdings, date, self.store)
    }

    pub fn allocation_by_security(&self, date: NaiveDate) -> Result<Allocation> {
        let valuation = self.valuation_at(date)?;
        Ok(allocation_by_security(&valuation, &self.securities()?))
    }

    pub fn allocation_by_account(&self, date: NaiveDate) -> Result<Allocation> {
        let transactions = self.transactions_until(Some(date))?;
        let accounts: Vec<Account> = self.scope_accounts().into_iter().cloned().collect();
        allocation_by_account(
            &transactions,
            &accounts,
            &self.securities()?,
            self.base_currency(),
            date,
            self.store,
            self.store,
            self.options(),
        )
    }

    /// Builds a rebalance plan; options change the target calculation itself.
    pub fn rebalance(
        &self,
        target: &AllocationTarget,
        date: NaiveDate,
        options: RebalanceOptions,
    ) -> Result<RebalancePlan> {
        let valuation = self.valuation_at(date)?;
        let nodes = self.store.taxonomy_nodes(&target.taxonomy_id)?;
        let subjects = self.taxonomy_subjects(&target.taxonomy_id, date)?;
        let assignments = self.taxonomy_assignments(&target.taxonomy_id)?;
        let allocation = allocation_by_taxonomy(&subjects, &nodes, &assignments);
        rebalance(
            &valuation,
            &allocation,
            target,
            &nodes,
            &assignments,
            &self.securities()?,
            &subjects,
            options,
        )
    }
}
