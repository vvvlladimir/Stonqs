//! Which subjects make up one node, the unclassified rest, or the whole portfolio.

use super::share;
use super::taxonomy::{Assignment, SubjectKind, TaxonomySubject, group_assignments};
use crate::model::TaxonomyNode;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Subject contribution within a taxonomy node, including its assigned fraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMember {
    /// Security ID or `cash:<account>:<currency>`.
    pub subject_id: String,
    pub kind: SubjectKind,
    pub symbol: String,
    pub name: String,
    /// Base-currency value assigned to this node.
    #[serde(with = "rust_decimal::serde::str")]
    pub value_base: Decimal,
    /// Weight within the node; excluded subjects have zero weight.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
    /// Full subject value used as the assignment denominator.
    #[serde(with = "rust_decimal::serde::str")]
    pub subject_value_base: Decimal,
    /// Assigned fraction of the subject; `0.6` means 60%.
    #[serde(with = "rust_decimal::serde::str")]
    pub assigned_share: Decimal,
    /// Excluded from this tree; its assignment is retained but ignored.
    pub excluded: bool,
}

/// Scope requested from [`allocation_members`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberScope<'a> {
    /// Whole portfolio; each subject contributes fully.
    Portfolio,
    /// Node and its subtree.
    Node(&'a str),
    /// Unassigned remainder `1 - sum(weights)`.
    Unclassified,
}

/// Returns subjects assigned to a scope, with values and within-scope weights.
/// Excluded subjects remain visible with zero weight and never enter the remainder.
pub fn allocation_members(
    subjects: &[TaxonomySubject],
    nodes: &[TaxonomyNode],
    assignments: &[Assignment],
    scope: MemberScope<'_>,
) -> Vec<NodeMember> {
    // Taxonomies are small; a simple breadth-like scan is clearer than a map.
    let subtree: Option<Vec<&str>> = match scope {
        MemberScope::Node(id) => {
            let mut ids: Vec<&str> = vec![id];
            let mut grew = true;
            while grew {
                grew = false;
                for node in nodes {
                    let parent = node.parent_id.as_deref();
                    if parent.is_some_and(|p| ids.contains(&p)) && !ids.contains(&node.id.as_str()) {
                        ids.push(&node.id);
                        grew = true;
                    }
                }
            }
            Some(ids)
        }
        _ => None,
    };

    let by_subject = group_assignments(assignments);

    let mut members: Vec<NodeMember> = subjects
        .iter()
        .filter_map(|subject| {
            let assigned: Decimal = by_subject
                .get(subject.key.as_str())
                .into_iter()
                .flatten()
                .filter(|a| match &subtree {
                    Some(ids) => ids.contains(&a.node_id.as_str()),
                    None => true,
                })
                .map(|a| a.weight)
                .sum();

            let share_of_subject = match scope {
                MemberScope::Portfolio => Decimal::ONE,
                MemberScope::Node(_) => assigned,
                // Validation rejects sums above one; clamp defensively.
                MemberScope::Unclassified if subject.excluded => Decimal::ZERO,
                MemberScope::Unclassified => (Decimal::ONE - assigned).max(Decimal::ZERO),
            };
            if share_of_subject.is_zero() {
                return None;
            }

            Some(NodeMember {
                subject_id: subject.key.clone(),
                kind: subject.kind,
                symbol: subject.symbol.clone(),
                name: subject.name.clone(),
                value_base: subject.value_base * share_of_subject,
                weight: Decimal::ZERO,
                subject_value_base: subject.value_base,
                assigned_share: share_of_subject,
                excluded: subject.excluded,
            })
        })
        .collect();

    // Weights are normalized within the requested scope; excluded subjects stay zero.
    let total: Decimal = members.iter().filter(|m| !m.excluded).map(|m| m.value_base).sum();
    for m in &mut members {
        if !m.excluded {
            m.weight = share(m.value_base, total);
        }
    }
    // Show included subjects first, sorted by value; excluded subjects go last.
    members.sort_by(|a, b| {
        a.excluded
            .cmp(&b.excluded)
            .then_with(|| b.value_base.cmp(&a.value_base))
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    members
}
