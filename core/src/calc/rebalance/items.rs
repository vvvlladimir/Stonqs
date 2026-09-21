//! Turning resolved nodes into the plan's rows, then settling the weights each row reports.

use super::moves::{NodeBudget, NodeMoves, plan_moves};
use super::nodes::{Node, subtree_ids};
use super::{Prices, RebalanceItem, RebalanceOptions};
use crate::calc::Assignment;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Builds one row per targeted node, and the purchase budgets the leftover pass will top up.
/// Returns the rows, those budgets, and how much of the portfolio already sits on target.
pub(super) fn build(
    plan_nodes: Vec<Node<'_>>,
    assignments: &[Assignment],
    prices: &Prices<'_>,
    options: RebalanceOptions,
    scale: Decimal,
    total: Decimal,
) -> (Vec<RebalanceItem>, Vec<NodeBudget>, Decimal) {
    let mut items = Vec::with_capacity(plan_nodes.len());
    let mut on_target = Decimal::ZERO;
    let mut budgets: Vec<NodeBudget> = Vec::new();

    for node in plan_nodes {
        let Node {
            weight: w,
            absolute,
            parent_base,
            leaf,
            bucket,
            current,
            target_base,
            drift,
        } = node;
        if leaf {
            on_target += current;
        }

        // Buy-only mode leaves overweights untouched and scales shortfalls to new cash.
        let actionable = if drift.is_sign_negative() && !options.allow_sell {
            Decimal::ZERO
        } else if drift.is_sign_positive() {
            drift * scale
        } else {
            drift
        };

        let subtree = bucket.map(subtree_ids).unwrap_or_default();
        let moves = if leaf {
            plan_moves(&subtree, actionable, assignments, prices)
        } else {
            NodeMoves::default()
        };
        let NodeMoves {
            trades,
            deposits,
            candidates,
            tradable_share,
        } = moves;
        if actionable.is_sign_positive() && !candidates.is_empty() {
            budgets.push(NodeBudget {
                item: items.len(),
                // Only the tradable share enters the purchase budget; cash subjects keep theirs.
                budget: tradable_share,
                shortfall: drift,
                spent: trades
                    .iter()
                    .map(|t| t.estimated_base)
                    .filter(|v| v.is_sign_positive())
                    .sum::<Decimal>(),
                candidates,
            });
        }

        items.push(RebalanceItem {
            node_id: w.node_id.clone(),
            label: bucket
                .map(|b| b.label.clone())
                .unwrap_or_else(|| w.node_id.clone()),
            current_base: current,
            current_weight: if total.is_zero() {
                Decimal::ZERO
            } else {
                current / total
            },
            target_weight: absolute,
            relative_target_weight: w.weight,
            relative_current_weight: if parent_base.is_zero() {
                Decimal::ZERO
            } else {
                current / parent_base
            },
            target_base,
            drift_base: drift,
            drift_weight: absolute
                - if total.is_zero() {
                    Decimal::ZERO
                } else {
                    current / total
                },
            leaf,
            trades,
            deposits,
        });
    }
    (items, budgets, on_target)
}

/// A security's weight after the plan depends on every trade in every node, so it can only be
/// filled once all of them are known. Same for a deposit and its account.
pub(super) fn settle_weights(
    items: &mut [RebalanceItem],
    total: Decimal,
    prices: &Prices<'_>,
) -> (Decimal, Decimal) {
    let value_of = &prices.value_of;
    // Aggregate each security's trades before assigning its final weight to every row.
    let mut delta: HashMap<String, Decimal> = HashMap::new();
    let mut sell = Decimal::ZERO;
    let mut cash_used = Decimal::ZERO;
    for item in items.iter() {
        for trade in &item.trades {
            *delta.entry(trade.security_id.clone()).or_default() += trade.estimated_base;
            if trade.estimated_base.is_sign_negative() {
                sell -= trade.estimated_base;
            } else {
                cash_used += trade.estimated_base;
            }
        }
        // Deposits are cash movements, not purchases or sales.
        for deposit in &item.deposits {
            *delta.entry(deposit.subject_id.clone()).or_default() += deposit.amount_base;
        }
    }
    for item in items.iter_mut() {
        for trade in &mut item.trades {
            if total.is_zero() {
                continue;
            }
            let current = value_of
                .get(trade.security_id.as_str())
                .copied()
                .unwrap_or(Decimal::ZERO);
            let moved = delta.get(&trade.security_id).copied().unwrap_or(Decimal::ZERO);
            trade.weight_after = (current + moved) / total;
        }
        for deposit in &mut item.deposits {
            if total.is_zero() {
                continue;
            }
            let moved = delta.get(&deposit.subject_id).copied().unwrap_or(Decimal::ZERO);
            deposit.weight_after = (deposit.current_base + moved) / total;
        }
    }
    (sell, cash_used)
}
