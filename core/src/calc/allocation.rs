use super::{Holdings, HoldingsOptions, PortfolioValuation, ordered_events};
use crate::calc::holdings::Event;
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::PriceLookup;
use crate::model::{
    Account, CashClassification, Security, SecurityClassification, TaxonomyNode, Transaction,
    cash_subject_key,
};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Stable key for cash buckets across allocation views.
pub const CASH_KEY: &str = "CASH";
/// Stable key for unclassified value.
pub const UNCLASSIFIED_KEY: &str = "UNCLASSIFIED";

/// Allocation bucket with value, weight, and optional children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationBucket {
    /// Source identifier: node, currency, account, security, or service key.
    pub key: String,
    pub label: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub value_base: Decimal,
    /// Share of total portfolio value; `0.25` means 25%.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
    /// Child buckets, used by taxonomy-tree allocations.
    pub children: Vec<AllocationBucket>,
}

/// Portfolio allocation. Top-level weights include explicit cash and
/// unclassified buckets where needed; taxonomy uses an instrument-only denominator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
    pub buckets: Vec<AllocationBucket>,
}

impl Allocation {
    /// Sum of top-level weights, useful for checks and tests.
    pub fn total_weight(&self) -> Decimal {
        self.buckets.iter().map(|b| b.weight).sum()
    }

    /// Finds a key recursively, including nested buckets.
    pub fn find(&self, key: &str) -> Option<&AllocationBucket> {
        fn walk<'a>(buckets: &'a [AllocationBucket], key: &str) -> Option<&'a AllocationBucket> {
            for b in buckets {
                if b.key == key {
                    return Some(b);
                }
                if let Some(found) = walk(&b.children, key) {
                    return Some(found);
                }
            }
            None
        }
        walk(&self.buckets, key)
    }
}

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

    let mut children_of: HashMap<Option<&str>, Vec<&TaxonomyNode>> = HashMap::new();
    for n in nodes {
        children_of.entry(n.parent_id.as_deref()).or_default().push(n);
    }

    fn build(
        node: &TaxonomyNode,
        children_of: &HashMap<Option<&str>, Vec<&TaxonomyNode>>,
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

    let mut children_of: HashMap<Option<&str>, Vec<&TaxonomyNode>> = HashMap::new();
    for n in nodes {
        children_of.entry(n.parent_id.as_deref()).or_default().push(n);
    }

    fn build(
        node: &TaxonomyNode,
        children_of: &HashMap<Option<&str>, Vec<&TaxonomyNode>>,
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
        // Stable descending order keeps treemap layout compact and predictable.
        children.sort_by(|a, b| {
            b.value_base
                .cmp(&a.value_base)
                .then_with(|| a.label.cmp(&b.label))
        });
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
    buckets.sort_by(|a, b| {
        b.value_base
            .cmp(&a.value_base)
            .then_with(|| a.label.cmp(&b.label))
    });

    if !unclassified.is_empty() {
        let value: Decimal = unclassified.iter().map(|c| c.value_base).sum();
        unclassified.sort_by_key(|b| std::cmp::Reverse(b.value_base));
        buckets.push(AllocationBucket {
            key: UNCLASSIFIED_KEY.to_string(),
            // A generated bucket has no name of its own: the key is the label, and the UI
            // writes the word in the user's language.
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

fn group_assignments(assignments: &[Assignment]) -> HashMap<&str, Vec<&Assignment>> {
    let mut out: HashMap<&str, Vec<&Assignment>> = HashMap::new();
    for a in assignments {
        out.entry(&a.subject_id).or_default().push(a);
    }
    out
}

/// Allocates securities by quote currency and cash by its holding currency.
pub fn allocation_by_currency(
    valuation: &PortfolioValuation,
    holdings: &Holdings,
    date: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<Allocation> {
    let mut by_currency: BTreeMap<Currency, Decimal> = BTreeMap::new();
    for p in &valuation.positions {
        *by_currency.entry(p.currency.clone()).or_default() += p.market_value_base;
    }
    for (currency, amount) in &holdings.cash {
        if amount.is_zero() {
            continue;
        }
        *by_currency.entry(currency.clone()).or_default() +=
            rates.convert(*amount, currency, &valuation.base_currency, date)?;
    }

    let total = valuation.total_value_base;
    let buckets = by_currency
        .into_iter()
        .map(|(currency, value)| AllocationBucket {
            key: currency.clone(),
            label: currency,
            value_base: value,
            weight: share(value, total),
            children: Vec::new(),
        })
        .collect();
    Ok(Allocation {
        total_base: total,
        buckets,
    })
}

/// Allocates by security, with cash in a separate bucket.
pub fn allocation_by_security(valuation: &PortfolioValuation, securities: &[Security]) -> Allocation {
    let names: HashMap<&str, &Security> = securities.iter().map(|s| (s.id.as_str(), s)).collect();
    let total = valuation.total_value_base;
    let mut buckets: Vec<AllocationBucket> = valuation
        .positions
        .iter()
        .map(|p| AllocationBucket {
            key: p.security_id.clone(),
            label: names
                .get(p.security_id.as_str())
                .map(|s| s.symbol.clone())
                .unwrap_or_else(|| p.security_id.clone()),
            value_base: p.market_value_base,
            weight: share(p.market_value_base, total),
            children: Vec::new(),
        })
        .collect();
    push_rest(&mut buckets, Decimal::ZERO, valuation.cash_base, total);
    Allocation {
        total_base: total,
        buckets,
    }
}

/// Allocates by account using per-account quantities and cash; cost basis remains portfolio-wide.
// Arguments represent independent calculation inputs; keep the public API explicit.
#[allow(clippy::too_many_arguments)]
pub fn allocation_by_account(
    transactions: &[Transaction],
    accounts: &[Account],
    securities: &[Security],
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Allocation> {
    let base = normalize_currency(base);
    let by_id: HashMap<&str, &Security> = securities.iter().map(|s| (s.id.as_str(), s)).collect();
    // Depot trades settle cash on the linked cash account, not on the depot.
    let settlement = super::balances::settlement_accounts(accounts);

    let mut quantities: BTreeMap<(String, String), Decimal> = BTreeMap::new();
    let mut cash: BTreeMap<(String, Currency), Decimal> = BTreeMap::new();

    for event in ordered_events(transactions, options.corporate_actions) {
        if event.date() > date {
            break;
        }
        match event {
            Event::Action(action) => {
                let factor = action.quantity_factor()?;
                for ((_, security_id), quantity) in quantities.iter_mut() {
                    if *security_id == action.security_id {
                        *quantity *= factor;
                    }
                }
            }
            Event::Tx(t) => {
                let sign = Decimal::from(t.kind.quantity_sign());
                if !sign.is_zero()
                    && let Some(sid) = &t.security_id
                {
                    *quantities.entry((t.account_id.clone(), sid.clone())).or_default() += sign * t.quantity;
                }
                let delta = t.cash_delta();
                if !delta.is_zero() {
                    let account_id = settlement
                        .get(t.account_id.as_str())
                        .map(|id| (*id).to_string())
                        .unwrap_or_else(|| t.account_id.clone());
                    *cash.entry((account_id, t.currency.clone())).or_default() += delta;
                }
            }
        }
    }

    let mut values: BTreeMap<String, Decimal> = BTreeMap::new();
    for ((account_id, security_id), quantity) in &quantities {
        if quantity.is_zero() {
            continue;
        }
        // A transaction security must exist in the supplied reference data.
        if !by_id.contains_key(security_id.as_str()) {
            return Err(Error::NotFound(format!("security {security_id}")));
        }
        let price = prices
            .price_as_of(security_id, date)?
            .ok_or_else(|| Error::MissingMarketData {
                kind: "price",
                key: security_id.clone(),
                date,
            })?;
        // Use the quote currency returned by the provider, not stale reference data.
        let value = rates.convert(*quantity * price.close, &price.currency, &base, date)?;
        *values.entry(account_id.clone()).or_default() += value;
    }
    for ((account_id, currency), amount) in &cash {
        if amount.is_zero() {
            continue;
        }
        *values.entry(account_id.clone()).or_default() += rates.convert(*amount, currency, &base, date)?;
    }

    let names: HashMap<&str, &Account> = accounts.iter().map(|a| (a.id.as_str(), a)).collect();
    let total: Decimal = values.values().sum();
    let buckets = values
        .into_iter()
        .map(|(account_id, value)| AllocationBucket {
            label: names
                .get(account_id.as_str())
                .map(|a| a.name.clone())
                .unwrap_or_else(|| account_id.clone()),
            key: account_id,
            value_base: value,
            weight: share(value, total),
            children: Vec::new(),
        })
        .collect();
    Ok(Allocation {
        total_base: total,
        buckets,
    })
}

/// Share of a total; zero is valid for an empty portfolio.
fn share(value: Decimal, total: Decimal) -> Decimal {
    if total.is_zero() {
        Decimal::ZERO
    } else {
        value / total
    }
}

/// Adds non-zero unclassified and cash buckets.
fn push_rest(buckets: &mut Vec<AllocationBucket>, unclassified: Decimal, cash: Decimal, total: Decimal) {
    if !unclassified.is_zero() {
        buckets.push(AllocationBucket {
            key: UNCLASSIFIED_KEY.to_string(),
            // A generated bucket has no name of its own: the key is the label, and the UI
            // writes the word in the user's language.
            label: UNCLASSIFIED_KEY.to_string(),
            value_base: unclassified,
            weight: share(unclassified, total),
            children: Vec::new(),
        });
    }
    if !cash.is_zero() {
        buckets.push(AllocationBucket {
            key: CASH_KEY.to_string(),
            label: CASH_KEY.to_string(),
            value_base: cash,
            weight: share(cash, total),
            children: Vec::new(),
        });
    }
}

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
