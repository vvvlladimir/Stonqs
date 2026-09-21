use super::TaxonomyNode;
use crate::error::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetWeight {
    pub node_id: String,
    /// Share of its **parent**, not of the whole portfolio — see CLAUDE.md invariants.
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
}

/// Target allocation attached to a taxonomy tree, not to securities directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationTarget {
    pub id: String,
    pub portfolio_id: String,
    pub taxonomy_id: String,
    pub name: String,
    pub weights: Vec<TargetWeight>,
}

impl AllocationTarget {
    pub fn new(portfolio_id: &str, taxonomy_id: &str, name: impl Into<String>) -> Self {
        AllocationTarget {
            id: super::new_id(),
            portfolio_id: portfolio_id.to_string(),
            taxonomy_id: taxonomy_id.to_string(),
            name: name.into(),
            weights: Vec::new(),
        }
    }

    pub fn with_weight(mut self, node_id: &str, weight: Decimal) -> Self {
        self.weights.push(TargetWeight {
            node_id: node_id.to_string(),
            weight,
        });
        self
    }

    /// Sum as recorded; only meaningful within one parent.
    pub fn total_weight(&self) -> Decimal {
        self.weights.iter().map(|w| w.weight).sum()
    }

    pub fn weight_of(&self, node_id: &str) -> Option<Decimal> {
        self.weights
            .iter()
            .find(|w| w.node_id == node_id)
            .map(|w| w.weight)
    }

    /// Absolute share per node: product along the path. Ancestors with no weight of their own are skipped, not treated as zero.
    pub fn absolute_weights(&self, nodes: &[TaxonomyNode]) -> BTreeMap<String, Decimal> {
        let by_id = index(nodes);
        let mine: HashMap<&str, Decimal> = self
            .weights
            .iter()
            .map(|w| (w.node_id.as_str(), w.weight))
            .collect();
        self.weights
            .iter()
            .map(|w| {
                let mut factor = w.weight;
                for id in ancestors(&by_id, &w.node_id) {
                    if let Some(parent) = mine.get(id) {
                        factor *= *parent;
                    }
                }
                (w.node_id.clone(), factor)
            })
            .collect()
    }

    /// Deepest weighted nodes — only these divide money; a weighted parent gets no trades of its own.
    pub fn leaf_node_ids(&self, nodes: &[TaxonomyNode]) -> BTreeSet<String> {
        let by_id = index(nodes);
        let mut leaves: BTreeSet<String> = self.weights.iter().map(|w| w.node_id.clone()).collect();
        for w in &self.weights {
            for id in ancestors(&by_id, &w.node_id) {
                leaves.remove(id);
            }
        }
        leaves
    }

    pub fn validate(&self) -> Result<()> {
        for w in &self.weights {
            if w.weight <= Decimal::ZERO || w.weight > Decimal::ONE {
                return Err(Error::Invalid(format!(
                    "target weight must be in (0, 1], got {} for node {}",
                    w.weight, w.node_id
                )));
            }
        }
        Ok(())
    }

    /// Full check: weight range plus sibling-sum per parent.
    pub fn validate_tree(&self, nodes: &[TaxonomyNode]) -> Result<()> {
        self.validate()?;
        let by_id = index(nodes);
        let mut sums: BTreeMap<Option<&str>, Decimal> = BTreeMap::new();
        for w in &self.weights {
            let node = by_id
                .get(w.node_id.as_str())
                .ok_or_else(|| Error::Invalid(format!("target references unknown node {}", w.node_id)))?;
            *sums.entry(node.parent_id.as_deref()).or_default() += w.weight;
        }
        for (parent, total) in sums {
            if total > Decimal::ONE {
                let whose = match parent {
                    Some(id) => format!("inside node {id}"),
                    None => "at the top level".to_string(),
                };
                return Err(Error::Invalid(format!(
                    "target weights sum to {total} {whose}, which is more than 1"
                )));
            }
        }
        Ok(())
    }
}

fn index(nodes: &[TaxonomyNode]) -> HashMap<&str, &TaxonomyNode> {
    nodes.iter().map(|n| (n.id.as_str(), n)).collect()
}

/// Path upward from a node, excluding itself; bounded by tree size so a cyclic parent link can't hang.
fn ancestors<'a>(by_id: &HashMap<&'a str, &'a TaxonomyNode>, node_id: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut current = by_id.get(node_id).and_then(|n| n.parent_id.as_deref());
    while let Some(id) = current {
        let Some((key, node)) = by_id.get_key_value(id) else {
            break;
        };
        if out.contains(key) {
            break;
        }
        out.push(*key);
        current = node.parent_id.as_deref();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Taxonomy, TaxonomyKind};
    use rust_decimal_macros::dec;

    /// 0.42 × 0.88 = 0.3696 (core); 0.42 × 0.12 = 0.0504 (defensive); sum = 0.42 = parent's own weight.
    #[test]
    fn relative_weights_multiply_along_the_path() {
        let tax = Taxonomy::new("Classes", TaxonomyKind::AssetClass);
        let equities = TaxonomyNode::root(&tax.id, "Equities");
        let core = TaxonomyNode::child(&equities, "Core");
        let defensive = TaxonomyNode::child(&equities, "Defensive");
        let nodes = vec![equities.clone(), core.clone(), defensive.clone()];

        let target = AllocationTarget::new("p", &tax.id, "Target")
            .with_weight(&equities.id, dec!(0.42))
            .with_weight(&core.id, dec!(0.88))
            .with_weight(&defensive.id, dec!(0.12));

        let abs = target.absolute_weights(&nodes);
        assert_eq!(abs[&core.id], dec!(0.3696));
        assert_eq!(abs[&defensive.id], dec!(0.0504));
        assert_eq!(abs[&equities.id], dec!(0.42));
        assert_eq!(abs[&core.id] + abs[&defensive.id], abs[&equities.id]);

        let leaves = target.leaf_node_ids(&nodes);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.contains(&core.id) && !leaves.contains(&equities.id));

        target.validate_tree(&nodes).unwrap();
    }

    /// 0.8 + 0.4 as siblings is an error; 0.88 + 0.12 inside a 0.42 node isn't, even though the total is 1.42.
    #[test]
    fn siblings_are_checked_apart_from_the_whole() {
        let tax = Taxonomy::new("Classes", TaxonomyKind::AssetClass);
        let a = TaxonomyNode::root(&tax.id, "A");
        let b = TaxonomyNode::root(&tax.id, "B");
        let inner = TaxonomyNode::child(&a, "A1");
        let nodes = vec![a.clone(), b.clone(), inner.clone()];

        let broken = AllocationTarget::new("p", &tax.id, "broken")
            .with_weight(&a.id, dec!(0.8))
            .with_weight(&b.id, dec!(0.4));
        assert!(broken.validate_tree(&nodes).is_err());

        let ok = AllocationTarget::new("p", &tax.id, "target")
            .with_weight(&a.id, dec!(0.42))
            .with_weight(&inner.id, dec!(1.0));
        assert_eq!(ok.total_weight(), dec!(1.42));
        ok.validate_tree(&nodes).unwrap();
    }
}
