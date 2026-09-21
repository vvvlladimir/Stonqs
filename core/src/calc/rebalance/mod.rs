//! Rebalancing: what a target tree says the portfolio should hold, against what it does hold.
//!
//! The options change what is *computed*, not what is displayed: new cash is added to the total
//! before targets are derived, and buy-only mode leaves overweights alone
//! (`.claude/rules/taxonomy-and-rebalance.md`).

mod budget;
mod items;
mod moves;
mod nodes;

use super::{Allocation, Assignment, PortfolioValuation, SubjectKind, TaxonomySubject};
use crate::error::Result;
use crate::model::{AllocationTarget, Security, TaxonomyNode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One security trade in a rebalance plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceTrade {
    pub security_id: String,
    pub symbol: String,
    /// Signed quantity: `+` buy, `-` sell, rounded to the tradable step.
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Estimated base-currency cost or proceeds.
    #[serde(with = "rust_decimal::serde::str")]
    pub estimated_base: Decimal,
    /// Base-currency unit price used by the calculation.
    #[serde(with = "rust_decimal::serde::str")]
    pub price_base: Decimal,
    /// Security's final portfolio weight after all its planned trades.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight_after: Decimal,
}

/// Cash amount needed for a taxonomy node to reach its target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashDeposit {
    /// Cash key `cash:<account>:<currency>`.
    pub subject_id: String,
    pub account_id: String,
    pub currency: String,
    /// Account label shown in allocation.
    pub label: String,
    /// Current base-currency value.
    #[serde(with = "rust_decimal::serde::str")]
    pub current_base: Decimal,
    /// Base-currency amount to deposit (`+`) or redirect to securities (`-`).
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
    /// Account's final portfolio weight after all deposits.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight_after: Decimal,
}

/// Target drift and planned actions for one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceItem {
    pub node_id: String,
    pub label: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub current_base: Decimal,
    /// Current share of the whole portfolio.
    #[serde(with = "rust_decimal::serde::str")]
    pub current_weight: Decimal,
    /// Absolute target share: product of weights along the path.
    #[serde(with = "rust_decimal::serde::str")]
    pub target_weight: Decimal,
    /// Target share within the nearest targeted parent.
    #[serde(with = "rust_decimal::serde::str")]
    pub relative_target_weight: Decimal,
    /// Current share using the nearest targeted parent's value as denominator.
    #[serde(with = "rust_decimal::serde::str")]
    pub relative_current_weight: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub target_base: Decimal,
    /// `target_base - current_base`; positive means underweight.
    #[serde(with = "rust_decimal::serde::str")]
    pub drift_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub drift_weight: Decimal,
    /// Leaf nodes receive trades; parent-only nodes report aggregate drift.
    pub leaf: bool,
    pub trades: Vec<RebalanceTrade>,
    /// Per-account cash changes for this node, alongside security trades.
    #[serde(default)]
    pub deposits: Vec<CashDeposit>,
}

/// Rebalance inputs that affect the calculation, not just presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceOptions {
    /// Cash added before target values are calculated.
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_to_invest: Decimal,
    /// Whether overweight positions may be sold.
    pub allow_sell: bool,
}

impl Default for RebalanceOptions {
    /// Default: rebalance existing value with sales allowed.
    fn default() -> Self {
        RebalanceOptions {
            cash_to_invest: Decimal::ZERO,
            allow_sell: true,
        }
    }
}

/// Complete rebalance plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlan {
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
    pub items: Vec<RebalanceItem>,
    /// Value outside targets: unclassified subjects and unassigned new cash.
    #[serde(with = "rust_decimal::serde::str")]
    pub off_target_base: Decimal,
    /// Cash spent on purchases, after step rounding.
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_used_base: Decimal,
    /// Proceeds from sales, reported as a positive amount.
    #[serde(with = "rust_decimal::serde::str")]
    pub sell_base: Decimal,
    /// Cash left: `cash_to_invest + sales - purchases`; may be negative.
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_left_base: Decimal,
}

/// Everything the plan reads off the valuation, gathered once rather than looked up per node.
/// A unit price is in base currency — the quote price times its own rate — and the quantity step
/// is the one the broker was observed to use, not the one the instrument kind implies.
pub(super) struct Prices<'a> {
    pub securities: HashMap<&'a str, &'a Security>,
    pub unit_price: HashMap<&'a str, Decimal>,
    pub value_of: HashMap<&'a str, Decimal>,
    pub step: HashMap<&'a str, Option<Decimal>>,
    /// Cash subjects share shortfall allocation with securities; execution is a deposit.
    pub cash: HashMap<&'a str, &'a TaxonomySubject>,
}

impl<'a> Prices<'a> {
    fn of(
        valuation: &'a PortfolioValuation,
        securities: &'a [Security],
        subjects: &'a [TaxonomySubject],
    ) -> Self {
        Prices {
            securities: securities.iter().map(|s| (s.id.as_str(), s)).collect(),
            unit_price: valuation
                .positions
                .iter()
                .map(|p| (p.security_id.as_str(), p.price * p.fx_rate))
                .collect(),
            value_of: valuation
                .positions
                .iter()
                .map(|p| (p.security_id.as_str(), p.market_value_base))
                .collect(),
            step: valuation
                .positions
                .iter()
                .map(|p| (p.security_id.as_str(), p.observed_quantity_step))
                .collect(),
            cash: subjects
                .iter()
                .filter(|s| s.kind == SubjectKind::Cash && !s.excluded)
                .map(|s| (s.key.as_str(), s))
                .collect(),
        }
    }
}

/// Computes target drift and trades, flooring quantities to observed steps.
/// New cash affects targets; nested parents report drift but do not trade.
#[allow(clippy::too_many_arguments)]
pub fn rebalance(
    valuation: &PortfolioValuation,
    allocation: &Allocation,
    target: &AllocationTarget,
    nodes: &[TaxonomyNode],
    assignments: &[Assignment],
    securities: &[Security],
    subjects: &[TaxonomySubject],
    options: RebalanceOptions,
) -> Result<RebalancePlan> {
    target.validate_tree(nodes)?;
    // Targets use allocated value plus new cash; cash is not already in taxonomy buckets.
    let total = allocation.total_base + options.cash_to_invest;
    let prices = Prices::of(valuation, securities, subjects);

    let plan_nodes = nodes::plan(allocation, target, nodes, total);
    // Scale shortfalls only among the same leaf nodes that spend the cash.
    let scale = nodes::purchase_scale(
        &plan_nodes
            .iter()
            .filter(|n| n.leaf)
            .map(|n| n.drift)
            .collect::<Vec<_>>(),
        options,
    );

    let (mut items, mut budgets, on_target) =
        items::build(plan_nodes, assignments, &prices, options, scale, total);

    // Spend remaining cash on additional whole trading steps.
    budget::spend_leftover(&mut items, &mut budgets, options.cash_to_invest);

    let (sell, cash_used) = items::settle_weights(&mut items, total, &prices);

    Ok(RebalancePlan {
        total_base: total,
        items,
        off_target_base: total - on_target,
        cash_used_base: cash_used,
        sell_base: sell,
        cash_left_base: options.cash_to_invest + sell - cash_used,
    })
}

#[cfg(test)]
mod tests;
