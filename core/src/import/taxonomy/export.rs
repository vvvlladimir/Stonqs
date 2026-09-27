//! A tree back out as the same CSV the import reads.

use super::{node_path, quote};
use crate::model::{AllocationTarget, Security, SecurityClassification, Taxonomy, TaxonomyNode};
use rust_decimal::Decimal;

pub fn taxonomy_to_csv(
    taxonomy: &Taxonomy,
    nodes: &[TaxonomyNode],
    classifications: &[SecurityClassification],
    securities: &[Security],
    target: Option<&AllocationTarget>,
) -> String {
    // The tree's name, the deepest path, and the security's own name one level below it.
    let levels = nodes.iter().map(|n| node_path(n, nodes).len()).max().unwrap_or(0) + 2;

    let mut out = String::new();
    for i in 1..=levels {
        out.push_str(&format!("Levels {i},"));
    }
    out.push_str("Weight,Allocation,Symbol,ISIN,Name\n");

    let mut write =
        |path: &[String], weight: &str, allocation: &str, symbol: &str, isin: &str, name: &str| {
            let mut cells: Vec<String> = vec![taxonomy.name.clone()];
            cells.extend(path.iter().cloned());
            while cells.len() < levels {
                cells.push(String::new());
            }
            let mut line: Vec<String> = cells.iter().map(|c| quote(c)).collect();
            for extra in [weight, allocation, symbol, isin, name] {
                line.push(quote(extra));
            }
            out.push_str(&line.join(","));
            out.push('\n');
        };

    fn walk(
        parent: Option<&str>,
        nodes: &[TaxonomyNode],
        classifications: &[SecurityClassification],
        securities: &[Security],
        target: Option<&AllocationTarget>,
        write: &mut impl FnMut(&[String], &str, &str, &str, &str, &str),
    ) {
        let mut children: Vec<&TaxonomyNode> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == parent)
            .collect();
        children.sort_by_key(|n| (n.rank, n.name.clone()));
        for node in children {
            let path: Vec<String> = node_path(node, nodes).into_iter().map(str::to_string).collect();
            let allocation = target
                .and_then(|t| t.weights.iter().find(|w| w.node_id == node.id))
                .map(|w| percent(w.weight))
                .unwrap_or_default();
            write(&path, "", &allocation, "", "", "");

            let mut mine: Vec<&SecurityClassification> =
                classifications.iter().filter(|c| c.node_id == node.id).collect();
            mine.sort_by_key(|c| std::cmp::Reverse(c.weight));
            for c in mine {
                let security = securities.iter().find(|s| s.id == c.security_id);
                let name = security.map(|s| s.name.clone()).unwrap_or_default();
                let symbol = security.map(|s| s.symbol.clone()).unwrap_or_default();
                let isin = security.and_then(|s| s.isin.clone()).unwrap_or_default();
                let mut deeper = path.clone();

                deeper.push(if name.is_empty() {
                    symbol.clone()
                } else {
                    name.clone()
                });
                write(&deeper, &percent(c.weight), "", &symbol, &isin, &name);
            }

            walk(Some(&node.id), nodes, classifications, securities, target, write);
        }
    }

    walk(None, nodes, classifications, securities, target, &mut write);
    out
}

fn percent(weight: Decimal) -> String {
    (weight * Decimal::ONE_HUNDRED)
        .round_dp(4)
        .normalize()
        .to_string()
}
