//! Goals and contribution limits: both intentions about the portfolio, so both read it whole
//! rather than through the account picker. See ADR-0068.

use crate::calc::{GoalProgress, LimitUsage, goal_progress, limit_usage};
use crate::error::Result;
use crate::model::{ContributionLimit, Goal};
use chrono::NaiveDate;

use super::PortfolioAnalytics;

impl PortfolioAnalytics<'_> {
    /// Measures one goal at `as_of`. A goal carries the accounts it counts, so the reading is
    /// taken over those and never over the lens the user happens to be looking through; naming
    /// none means the whole portfolio.
    pub fn goal_progress(&self, goal: &Goal, as_of: NaiveDate) -> Result<GoalProgress> {
        let whole = PortfolioAnalytics::new(self.store, self.portfolio)?;
        let analytics = if goal.accounts.is_empty() {
            whole
        } else {
            whole.scoped_to(&goal.accounts)
        };
        let current = analytics.valuation_at(as_of)?.total_value_base;
        goal_progress(goal, current, self.base_currency(), as_of, self.store)
    }

    /// Reads one limit over the limit year `as_of` falls in. The ledger is the portfolio's:
    /// a scope rewrites transfer legs into external flows, which is exactly the distinction a
    /// contribution turns on.
    pub fn limit_usage(&self, limit: &ContributionLimit, as_of: NaiveDate) -> Result<LimitUsage> {
        let whole = PortfolioAnalytics::new(self.store, self.portfolio)?;
        let transactions = whole.transactions_until(Some(as_of))?;
        limit_usage(limit, &transactions, as_of, self.store)
    }
}
