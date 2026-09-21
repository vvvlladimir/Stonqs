//! The target tree, resolved against what is actually held.
//!
//! A weight is a share of its *parent*, so the absolute share is a product along the path and is
//! always derived, never stored. Only the deepest weighted nodes divide money: a weighted parent
//! gets its own row and its own drift, but no trades (`.claude/rules/taxonomy-and-rebalance.md`).

use super::RebalanceOptions;
use crate::calc::{Allocation, AllocationBucket};
use crate::model::{AllocationTarget, TargetWeight, TaxonomyNode};
use rust_decimal::Decimal;

/// One targeted node, with what it holds and what it is owed. New-cash allocation needs every
/// shortfall before any single share is known, so the whole tree is resolved before it is spent.
pub(super) struct Node<'a> {
    pub weight: &'a TargetWeight,
    pub absolute: Decimal,
    pub parent_base: Decimal,
    pub leaf: bool,
    pub bucket: Option<&'a AllocationBucket>,
    pub current: Decimal,
    pub target_base: Decimal,
    pub drift: Decimal,
}

pub(super) fn plan<'a>(
    allocation: &'a Allocation,
    target: &'a AllocationTarget,
    nodes: &[TaxonomyNode],
    total: Decimal,
) -> Vec<Node<'a>> {
    // Convert relative target weights to absolute values and parent denominators.
    let absolute = target.absolute_weights(nodes);
    let leaves = target.leaf_node_ids(nodes);

    let plan_nodes: Vec<Node<'_>> = target
        .weights
        .iter()
        .map(|w| {
            let bucket = allocation.find(&w.node_id);
            let current = bucket.map(|b| b.value_base).unwrap_or(Decimal::ZERO);
            let absolute = absolute.get(&w.node_id).copied().unwrap_or(w.weight);
            let target_base = total * absolute;
            let parent_base = weighted_parent(&w.node_id, target, nodes)
                .and_then(|id| allocation.find(&id))
                .map(|b| b.value_base)
                .unwrap_or(total);
            Node {
                weight: w,
                absolute,
                parent_base,
                leaf: leaves.contains(&w.node_id),
                bucket,
                current,
                target_base,
                drift: target_base - current,
            }
        })
        .collect();
    plan_nodes
}

/// Scales positive drifts so buy-only purchases fit within new cash.
pub(super) fn purchase_scale(drifts: &[Decimal], options: RebalanceOptions) -> Decimal {
    if options.allow_sell {
        return Decimal::ONE;
    }
    let shortfall: Decimal = drifts.iter().filter(|d| d.is_sign_positive()).sum();
    if shortfall <= options.cash_to_invest {
        return Decimal::ONE;
    }
    if shortfall.is_zero() {
        return Decimal::ZERO;
    }
    options.cash_to_invest / shortfall
}

/// Finds the nearest targeted ancestor used for relative weights.
fn weighted_parent(node_id: &str, target: &AllocationTarget, nodes: &[TaxonomyNode]) -> Option<String> {
    let mut current = nodes.iter().find(|n| n.id == node_id)?.parent_id.clone();
    while let Some(id) = current {
        if target.weight_of(&id).is_some() {
            return Some(id);
        }
        current = nodes.iter().find(|n| n.id == id)?.parent_id.clone();
    }
    None
}

/// IDs of a node and all descendants.
pub(super) fn subtree_ids(bucket: &AllocationBucket) -> Vec<String> {
    let mut out = vec![bucket.key.clone()];
    for child in &bucket.children {
        out.extend(subtree_ids(child));
    }
    out
}
