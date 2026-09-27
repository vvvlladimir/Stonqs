//! Allocation through a user's classification tree; cash is a subject like a position.

use super::{Allocation, AllocationBucket, PortfolioValuation, UNCLASSIFIED_KEY, push_rest, share};
use crate::model::{CashClassification, Security, SecurityClassification, TaxonomyNode, cash_subject_key};
use crate::money::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind of taxonomy subject: security or account cash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubjectKind {
    Security,
    Cash,
}

/// A security position or one account-currency cash balance split by taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomySubject {
    /// Security ID or `cash:<account>:<currency>`.
    pub key: String,
    pub kind: SubjectKind,
    /// Short label: security symbol or currency.
    pub symbol: String,
    /// Long label: security or account name.
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub value_base: Decimal,
    /// Excluded from this tree, its denominator, and unclassified remainder.
    pub excluded: bool,
}

/// One account-currency cash balance, converted to the base currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashSubject {
    pub account_id: String,
    pub account_name: String,
    pub currency: Currency,
    pub value_base: Decimal,
}

/// Assignment of either a security or cash subject to a taxonomy node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub subject_id: String,
    pub node_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
}

impl From<&SecurityClassification> for Assignment {
    fn from(c: &SecurityClassification) -> Self {
        Assignment {
            subject_id: c.security_id.clone(),
            node_id: c.node_id.clone(),
            weight: c.weight,
        }
    }
}

impl From<&CashClassification> for Assignment {
    fn from(c: &CashClassification) -> Self {
        Assignment {
            subject_id: cash_subject_key(&c.account_id, &c.currency),
            node_id: c.node_id.clone(),
            weight: c.weight,
        }
    }
}

/// Builds taxonomy subjects from valued positions and non-zero cash balances.
pub fn taxonomy_subjects(
    valuation: &PortfolioValuation,
    securities: &[Security],
    cash: &[CashSubject],
    excluded: &[String],
) -> Vec<TaxonomySubject> {
    let names: HashMap<&str, &Security> = securities.iter().map(|s| (s.id.as_str(), s)).collect();
    let is_off = |key: &str| excluded.iter().any(|e| e == key);

    let mut out: Vec<TaxonomySubject> = valuation
        .positions
        .iter()
        .map(|p| {
            let security = names.get(p.security_id.as_str());
            TaxonomySubject {
                excluded: is_off(&p.security_id),
                key: p.security_id.clone(),
                kind: SubjectKind::Security,
                symbol: security
                    .map(|s| s.symbol.clone())
                    .unwrap_or_else(|| p.security_id.clone()),
                name: security.map(|s| s.name.clone()).unwrap_or_default(),
                value_base: p.market_value_base,
            }
        })
        .collect();

    out.extend(cash.iter().filter(|c| !c.value_base.is_zero()).map(|c| {
        let key = cash_subject_key(&c.account_id, &c.currency);
        TaxonomySubject {
            excluded: is_off(&key),
            key,
            kind: SubjectKind::Cash,
            symbol: c.currency.clone(),
            name: c.account_name.clone(),
            value_base: c.value_base,
        }
    }));
    out
}

/// Allocates subjects through taxonomy assignments. Each subject contributes
/// `value * weight`; the unassigned remainder is kept in `UNCLASSIFIED_KEY`.
pub fn allocation_by_taxonomy(
    subjects: &[TaxonomySubject],
    nodes: &[TaxonomyNode],
    assignments: &[Assignment],
) -> Allocation {
    let mut direct: HashMap<&str, Decimal> = HashMap::new();
    let by_subject = group_assignments(assignments);

    let mut unclassified = Decimal::ZERO;
    for subject in subjects.iter().filter(|s| !s.excluded) {
        let mut assigned = Decimal::ZERO;
        for a in by_subject.get(subject.key.as_str()).into_iter().flatten() {
            *direct.entry(a.node_id.as_str()).or_default() += subject.value_base * a.weight;
            assigned += a.weight;
        }
        // Input validation rejects weights above one; clamp defensively here.
        let rest = (Decimal::ONE - assigned).max(Decimal::ZERO);
        unclassified += subject.value_base * rest;
    }

    let children_of = children_of(nodes);

    fn build(
        node: &TaxonomyNode,
        children_of: &ChildrenOf<'_>,
        direct: &HashMap<&str, Decimal>,
        total: Decimal,
    ) -> AllocationBucket {
        let children: Vec<AllocationBucket> = children_of
            .get(&Some(node.id.as_str()))
            .into_iter()
            .flatten()
            .map(|child| build(child, children_of, direct, total))
            .collect();
        let value = direct.get(node.id.as_str()).copied().unwrap_or(Decimal::ZERO)
            + children.iter().map(|c| c.value_base).sum::<Decimal>();
        AllocationBucket {
            key: node.id.clone(),
            label: node.name.clone(),
            value_base: value,
            weight: share(value, total),
            children,
        }
    }

    let total: Decimal = subjects
        .iter()
        .filter(|s| !s.excluded)
        .map(|s| s.value_base)
        .sum();
    let mut buckets: Vec<AllocationBucket> = children_of
        .get(&None)
        .into_iter()
        .flatten()
        .map(|root| build(root, &children_of, &direct, total))
        .collect();

    push_rest(&mut buckets, unclassified, Decimal::ZERO, total);
    Allocation {
        total_base: total,
        buckets,
    }
}

/// Builds the full taxonomy tree with subject tiles under their assigned nodes.
/// Excluded subjects are omitted from both the tree and its denominator.
pub fn allocation_tree(
    subjects: &[TaxonomySubject],
    nodes: &[TaxonomyNode],
    assignments: &[Assignment],
) -> Allocation {
    let by_subject = group_assignments(assignments);
    let total: Decimal = subjects
        .iter()
        .filter(|s| !s.excluded)
        .map(|s| s.value_base)
        .sum();

    // Place subject tiles under assigned nodes; keep unassigned remainder separately.
    let mut members: HashMap<&str, Vec<AllocationBucket>> = HashMap::new();
    let mut unclassified: Vec<AllocationBucket> = Vec::new();
    for subject in subjects.iter().filter(|s| !s.excluded) {
        let mut assigned = Decimal::ZERO;
        for a in by_subject.get(subject.key.as_str()).into_iter().flatten() {
            assigned += a.weight;
            let value = subject.value_base * a.weight;
            members
                .entry(a.node_id.as_str())
                .or_default()
                .push(tile(subject, value, total));
        }
        let rest = (Decimal::ONE - assigned).max(Decimal::ZERO);
        if !rest.is_zero() {
            unclassified.push(tile(subject, subject.value_base * rest, total));
        }
    }

    let children_of = children_of(nodes);

    fn build(
        node: &TaxonomyNode,
        children_of: &ChildrenOf<'_>,
        members: &HashMap<&str, Vec<AllocationBucket>>,
        total: Decimal,
    ) -> AllocationBucket {
        let mut children: Vec<AllocationBucket> = children_of
            .get(&Some(node.id.as_str()))
            .into_iter()
            .flatten()
            .map(|child| build(child, children_of, members, total))
            .collect();
        children.extend(members.get(node.id.as_str()).cloned().unwrap_or_default());
        // Stable descending order keeps the treemap compact.
        children.sort_by(by_value_desc);
        let value = children.iter().map(|c| c.value_base).sum::<Decimal>();
        AllocationBucket {
            key: node.id.clone(),
            label: node.name.clone(),
            value_base: value,
            weight: share(value, total),
            children,
        }
    }

    let mut buckets: Vec<AllocationBucket> = children_of
        .get(&None)
        .into_iter()
        .flatten()
        .map(|root| build(root, &children_of, &members, total))
        .collect();
    buckets.sort_by(by_value_desc);

    if !unclassified.is_empty() {
        let value: Decimal = unclassified.iter().map(|c| c.value_base).sum();
        unclassified.sort_by_key(|b| std::cmp::Reverse(b.value_base));
        buckets.push(AllocationBucket {
            key: UNCLASSIFIED_KEY.to_string(),
            // The key is the label; the UI writes the word.
            label: UNCLASSIFIED_KEY.to_string(),
            value_base: value,
            weight: share(value, total),
            children: unclassified,
        });
    }

    Allocation {
        total_base: total,
        buckets,
    }
}

/// Builds a subject tile, preferring compact security symbols or account names.
fn tile(subject: &TaxonomySubject, value: Decimal, total: Decimal) -> AllocationBucket {
    let (short, long) = match subject.kind {
        SubjectKind::Security => (&subject.symbol, &subject.name),
        SubjectKind::Cash => (&subject.name, &subject.symbol),
    };
    AllocationBucket {
        key: subject.key.clone(),
        label: if short.trim().is_empty() {
            long.clone()
        } else {
            short.clone()
        },
        value_base: value,
        weight: share(value, total),
        children: Vec::new(),
    }
}

pub(super) fn group_assignments(assignments: &[Assignment]) -> HashMap<&str, Vec<&Assignment>> {
    let mut out: HashMap<&str, Vec<&Assignment>> = HashMap::new();
    for a in assignments {
        out.entry(&a.subject_id).or_default().push(a);
    }
    out
}

type ChildrenOf<'a> = HashMap<Option<&'a str>, Vec<&'a TaxonomyNode>>;

fn children_of(nodes: &[TaxonomyNode]) -> ChildrenOf<'_> {
    let mut out: ChildrenOf<'_> = HashMap::new();
    for n in nodes {
        out.entry(n.parent_id.as_deref()).or_default().push(n);
    }
    out
}

/// Largest first, then by label, so the layout is stable.
fn by_value_desc(a: &AllocationBucket, b: &AllocationBucket) -> std::cmp::Ordering {
    b.value_base
        .cmp(&a.value_base)
        .then_with(|| a.label.cmp(&b.label))
}
