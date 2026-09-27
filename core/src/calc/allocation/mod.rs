//! What the portfolio is made of, by any one dimension.

mod breakdowns;
mod members;
mod taxonomy;

pub use breakdowns::{allocation_by_account, allocation_by_currency, allocation_by_security};
pub use members::{MemberScope, NodeMember, allocation_members};
pub use taxonomy::{
    Assignment, CashSubject, SubjectKind, TaxonomySubject, allocation_by_taxonomy, allocation_tree,
    taxonomy_subjects,
};

use super::{Holdings, HoldingsOptions, PortfolioValuation};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
            // The key is the label; the UI writes the word.
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
