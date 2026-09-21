use super::{Assignment, Holdings, IncomeRecord, UNCLASSIFIED_KEY};
use crate::model::{TaxonomyNode, TransactionKind, cash_subject_key};
use chrono::Datelike;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Income rolled up over a group of events. Fees/taxes stay separate fields, same
/// reasoning as [`super::RealizedSummary`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomeSummary {
    pub events: usize,
    /// Gross, before tax withholding.
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    /// Negative for interest paid.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

impl IncomeSummary {
    pub fn add(&mut self, r: &IncomeRecord) {
        self.add_share(r, Decimal::ONE);
    }

    /// Adds the `weight` fraction of a payment. The event itself is counted whole wherever a
    /// fraction of it lands: 40% of a dividend is still one payment that node received.
    fn add_share(&mut self, r: &IncomeRecord, weight: Decimal) {
        self.events += 1;
        self.gross_base += r.gross_base * weight;
        self.taxes_base += r.taxes_base * weight;
        self.fees_base += r.fees_base * weight;
        self.net_base += r.net_base * weight;
    }
}

/// Income by calendar month, keyed `(year, month)`.
pub fn income_by_month(items: &[IncomeRecord]) -> BTreeMap<(i32, u32), IncomeSummary> {
    group(items, |r| Some((r.date.year(), r.date.month())))
}

/// Income by calendar year. A separate rollup, not summed client-side, because money
/// crosses the wire as a string precisely to keep `f64` out of the frontend.
pub fn income_by_year(items: &[IncomeRecord]) -> BTreeMap<i32, IncomeSummary> {
    group(items, |r| Some(r.date.year()))
}

/// Income by security. Events without one (e.g. account interest) are excluded.
pub fn income_by_security(items: &[IncomeRecord]) -> BTreeMap<String, IncomeSummary> {
    group(items, |r| r.security_id.clone())
}

pub fn income_by_kind(items: &[IncomeRecord]) -> BTreeMap<TransactionKind, IncomeSummary> {
    group(items, |r| Some(r.kind))
}

/// Income by month index (1..=12) with the years folded together — the calendar's
/// bottom row, where "March pays four times" becomes visible.
pub fn income_by_month_of_year(items: &[IncomeRecord]) -> BTreeMap<u32, IncomeSummary> {
    group(items, |r| Some(r.date.month()))
}

/// Income by year, split by kind: the same money as [`income_by_year`], divided the way
/// a stacked year bar needs it.
pub fn income_by_year_kind(items: &[IncomeRecord]) -> BTreeMap<(i32, TransactionKind), IncomeSummary> {
    group(items, |r| Some((r.date.year(), r.kind)))
}

/// Keeps only one kind of income; `None` keeps every kind. Filtering happens here, so a
/// caller asking for "dividends only" never re-adds money itself.
pub fn income_of_kind(items: &[IncomeRecord], kind: Option<TransactionKind>) -> Vec<IncomeRecord> {
    match kind {
        None => items.to_vec(),
        Some(kind) => items.iter().filter(|r| r.kind == kind).cloned().collect(),
    }
}

pub fn income_total(items: &[IncomeRecord]) -> IncomeSummary {
    let mut total = IncomeSummary::default();
    for r in items {
        total.add(r);
    }
    total
}

/// Income events in `[from, to]`, inclusive. Filtering lives here so the inclusive-bounds
/// rule isn't repeated on every screen.
pub fn income_between(
    holdings: &Holdings,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Vec<IncomeRecord> {
    holdings
        .income
        .iter()
        .filter(|r| r.date >= from && r.date <= to)
        .cloned()
        .collect()
}

/// One node of a classification tree with the income of its whole subtree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomeNode {
    /// Node id, or [`UNCLASSIFIED_KEY`].
    pub key: String,
    pub label: String,
    /// This node plus everything below it.
    pub summary: IncomeSummary,
    /// Share of the tree's net income. Interest charged is negative, so a weight is not
    /// guaranteed to sit between zero and one — a bar clamps, it does not renormalise.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
    pub children: Vec<IncomeNode>,
}

/// Income seen through a classification tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyIncome {
    /// Every payment that entered the tree; excluded subjects are not in it.
    pub total: IncomeSummary,
    pub nodes: Vec<IncomeNode>,
}

/// Splits income through a taxonomy: a payment follows the assignment of whoever paid it.
///
/// The payer is the instrument for a dividend and the account-currency cash subject for
/// interest, so an account classified as "Cash" reports its interest there. Splitting a
/// subject 60/40 splits its payments the same way; the unassigned remainder is
/// [`UNCLASSIFIED_KEY`]. An excluded subject leaves the tree and its total entirely — it is
/// not moved to the remainder, exactly as in [`super::allocation_by_taxonomy`].
///
/// Unlike an allocation, this reads no valuation: a security sold last spring paid its
/// dividend all the same, and the tree still knows what it was.
pub fn income_by_taxonomy(
    items: &[IncomeRecord],
    nodes: &[TaxonomyNode],
    assignments: &[Assignment],
    excluded: &[String],
) -> TaxonomyIncome {
    let mut by_subject: HashMap<&str, Vec<&Assignment>> = HashMap::new();
    for a in assignments {
        by_subject.entry(&a.subject_id).or_default().push(a);
    }

    let parent_of: HashMap<&str, &str> = nodes
        .iter()
        .filter_map(|n| Some((n.id.as_str(), n.parent_id.as_deref()?)))
        .collect();

    let mut direct: HashMap<&str, IncomeSummary> = HashMap::new();
    // A payment counted once per node it reached, ancestors included: rolling the children's
    // counts up instead would make one dividend split 60/40 read as two payments at the parent.
    let mut payments: HashMap<&str, usize> = HashMap::new();
    let mut unclassified = IncomeSummary::default();
    let mut total = IncomeSummary::default();

    for r in items {
        let subject = match &r.security_id {
            Some(id) => id.clone(),
            None => cash_subject_key(&r.account_id, &r.currency),
        };
        if excluded.contains(&subject) {
            continue;
        }
        total.add(r);

        let mut assigned = Decimal::ZERO;
        let mut reached: BTreeSet<&str> = BTreeSet::new();
        for a in by_subject.get(subject.as_str()).into_iter().flatten() {
            direct
                .entry(a.node_id.as_str())
                .or_default()
                .add_share(r, a.weight);
            assigned += a.weight;
            let mut node = a.node_id.as_str();
            while reached.insert(node) {
                match parent_of.get(node) {
                    Some(parent) => node = parent,
                    None => break,
                }
            }
        }
        for node in reached {
            *payments.entry(node).or_default() += 1;
        }
        // Input validation rejects weights above one; clamp defensively here.
        let rest = (Decimal::ONE - assigned).max(Decimal::ZERO);
        if !rest.is_zero() {
            unclassified.add_share(r, rest);
        }
    }

    let mut children_of: HashMap<Option<&str>, Vec<&TaxonomyNode>> = HashMap::new();
    for n in nodes {
        children_of.entry(n.parent_id.as_deref()).or_default().push(n);
    }

    fn build(
        node: &TaxonomyNode,
        children_of: &HashMap<Option<&str>, Vec<&TaxonomyNode>>,
        direct: &HashMap<&str, IncomeSummary>,
        payments: &HashMap<&str, usize>,
        total: Decimal,
    ) -> IncomeNode {
        let children: Vec<IncomeNode> = children_of
            .get(&Some(node.id.as_str()))
            .into_iter()
            .flatten()
            .map(|child| build(child, children_of, direct, payments, total))
            .collect();
        let mut summary = direct.get(node.id.as_str()).cloned().unwrap_or_default();
        for c in &children {
            summary.gross_base += c.summary.gross_base;
            summary.taxes_base += c.summary.taxes_base;
            summary.fees_base += c.summary.fees_base;
            summary.net_base += c.summary.net_base;
        }
        summary.events = payments.get(node.id.as_str()).copied().unwrap_or_default();
        IncomeNode {
            key: node.id.clone(),
            label: node.name.clone(),
            weight: share(summary.net_base, total),
            summary,
            children,
        }
    }

    let mut out: Vec<IncomeNode> = children_of
        .get(&None)
        .into_iter()
        .flatten()
        .map(|root| build(root, &children_of, &direct, &payments, total.net_base))
        .collect();

    if unclassified.events > 0 {
        out.push(IncomeNode {
            key: UNCLASSIFIED_KEY.to_string(),
            // A generated bucket has no name of its own: the key is the label, and the UI
            // writes the word in the user's language.
            label: UNCLASSIFIED_KEY.to_string(),
            weight: share(unclassified.net_base, total.net_base),
            summary: unclassified,
            children: Vec::new(),
        });
    }

    TaxonomyIncome { total, nodes: out }
}

fn share(value: Decimal, total: Decimal) -> Decimal {
    if total.is_zero() {
        Decimal::ZERO
    } else {
        value / total
    }
}

fn group<K: Ord, F: Fn(&IncomeRecord) -> Option<K>>(
    items: &[IncomeRecord],
    key: F,
) -> BTreeMap<K, IncomeSummary> {
    let mut out: BTreeMap<K, IncomeSummary> = BTreeMap::new();
    for r in items {
        if let Some(k) = key(r) {
            out.entry(k).or_default().add(r);
        }
    }
    out
}
