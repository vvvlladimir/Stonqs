//! Writing a taxonomy preview: nodes reused by name and place, so a re-import does not double.

use super::{TaxonomyPreview, node_path};
use crate::error::{Error, Result};
use crate::model::{AllocationTarget, SecurityClassification, TargetWeight, Taxonomy, TaxonomyNode, new_id};
use crate::storage::Store;
use std::collections::BTreeMap;

/// Extends `into` when given, else creates the tree; targets are written only for `portfolio_id`.
pub fn commit_taxonomy(
    store: &Store,
    preview: &TaxonomyPreview,
    into: Option<&str>,
    portfolio_id: Option<&str>,
) -> Result<Taxonomy> {
    let taxonomy = match into {
        Some(id) => store.get_taxonomy(id)?,
        None => {
            let taxonomy = Taxonomy::new(preview.name.trim(), preview.kind);
            if taxonomy.name.is_empty() {
                return Err(Error::Invalid("the classification has an empty name".into()));
            }
            store.save_taxonomy(&taxonomy)?;
            taxonomy
        }
    };

    let existing = store.taxonomy_nodes(&taxonomy.id)?;
    let mut ids: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for node in &existing {
        let path = node_path(node, &existing)
            .into_iter()
            .map(|name| name.trim().to_lowercase())
            .collect();
        ids.insert(path, node.id.clone());
    }
    let key = |path: &[String]| -> Vec<String> { path.iter().map(|p| p.trim().to_lowercase()).collect() };
    let mut rank = existing.len();

    for node in preview.nodes.iter() {
        for depth in 1..=node.path.len() {
            let path = key(&node.path[..depth]);
            if ids.contains_key(&path) {
                continue;
            }
            let parent = if depth > 1 {
                ids.get(&key(&node.path[..depth - 1])).cloned()
            } else {
                None
            };
            let saved = TaxonomyNode {
                id: new_id(),
                taxonomy_id: taxonomy.id.clone(),
                parent_id: parent,
                name: node.path[depth - 1].clone(),
                rank: rank as i64,

                color: None,
            };
            store.save_taxonomy_node(&saved)?;
            ids.insert(path, saved.id);
            rank += 1;
        }
    }

    for a in &preview.assignments {
        let (Some(security_id), Some(node_id)) = (a.security_id.as_deref(), ids.get(&key(&a.path))) else {
            continue;
        };
        store.save_classification(&SecurityClassification::new(security_id, node_id, a.weight))?;
    }

    if let Some(portfolio_id) = portfolio_id {
        let mut weights: Vec<TargetWeight> = preview
            .nodes
            .iter()
            .filter_map(|n| {
                let weight = n.target?;
                Some(TargetWeight {
                    node_id: ids.get(&key(&n.path))?.clone(),
                    weight,
                })
            })
            .collect();

        let existing_target = store
            .targets_for_portfolio(portfolio_id)?
            .into_iter()
            .find(|t| t.taxonomy_id == taxonomy.id);
        if let Some(previous) = &existing_target {
            for old in &previous.weights {
                if !weights.iter().any(|w| w.node_id == old.node_id) {
                    weights.push(old.clone());
                }
            }
        }
        if !weights.is_empty() {
            let target = AllocationTarget {
                id: existing_target.map(|t| t.id).unwrap_or_else(new_id),
                portfolio_id: portfolio_id.to_string(),
                taxonomy_id: taxonomy.id.clone(),
                name: format!("Target · {}", taxonomy.name),
                weights,
            };

            let saved_nodes = store.taxonomy_nodes(&taxonomy.id)?;
            if target.validate_tree(&saved_nodes).is_ok() {
                store.save_target(&target)?;
            }
        }
    }

    Ok(taxonomy)
}
