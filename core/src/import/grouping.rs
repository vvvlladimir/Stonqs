//! A taxonomy built from an instrument attribute.
//!
//! The source is a column of our own data rather than a file, but the product is the same plan a
//! CSV import produces ([`TaxonomyPreview`]) and it is written by the same [`commit_taxonomy`],
//! so node reuse and target handling have one implementation. See ADR-0032.
//!
//! [`commit_taxonomy`]: super::taxonomy::commit_taxonomy

use super::taxonomy::{PreviewAssignment, PreviewNode, TaxonomyCsvConfig, TaxonomyPreview};
use crate::error::{Error, Result};
use crate::model::{AttributeKind, Security, SecurityAttributeDef, TaxonomyKind};
use crate::storage::AttributeValues;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

/// Plans one node per distinct value of `def`, with every instrument carrying that value assigned
/// to it whole.
///
/// `classified` holds the instruments the target tree already places somewhere; they are planned
/// as skipped (`security_id: None`, `matched_by: "already_classified"`) rather than moved: the tree
/// is what the maths reads, so a split typed by hand outranks a value read off a column.
///
/// Groups are ordered by size, largest first — the node order is what the charts colour by, and the
/// big slice deserves the first slot.
pub fn group_by_attribute(
    def: &SecurityAttributeDef,
    securities: &[Security],
    values: &BTreeMap<String, AttributeValues>,
    classified: &BTreeSet<String>,
) -> Result<TaxonomyPreview> {
    // A node per distinct number or date is a list, not a classification: every instrument would
    // land in a group of its own. Grouping dates by year is a different feature.
    if def.kind != AttributeKind::Text {
        return Err(Error::Invalid(format!(
            "attribute {:?} is not text, so its values are not groups",
            def.name
        )));
    }

    let mut assignments = Vec::new();
    let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
    for (row, security) in securities.iter().enumerate() {
        let Some(value) = values
            .get(&security.id)
            .and_then(|v| v.get(&def.id))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        else {
            continue;
        };

        let skip = classified.contains(&security.id);
        if !skip {
            *sizes.entry(value.to_string()).or_default() += 1;
        }
        assignments.push(PreviewAssignment {
            row,
            path: vec![value.to_string()],
            label: security.name.clone(),
            symbol: security.symbol.clone(),
            isin: security.isin.clone().unwrap_or_default(),
            weight: Decimal::ONE,
            security_id: (!skip).then(|| security.id.clone()),
            matched_by: Some(if skip { "already_classified" } else { "attribute" }.to_string()),
        });
    }

    // A group everything in is already classified would be an empty node, so it is not planned.
    let mut nodes: Vec<(usize, String)> = sizes.into_iter().map(|(name, size)| (size, name)).collect();
    nodes.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    Ok(TaxonomyPreview {
        name: def.name.clone(),
        kind: TaxonomyKind::Custom,
        config: TaxonomyCsvConfig::default(),
        nodes: nodes
            .into_iter()
            .map(|(_, name)| PreviewNode {
                path: vec![name],
                target: None,
            })
            .collect(),
        assignments,
        problems: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::group_by_attribute;
    use crate::model::{AttributeKind, Security, SecurityAttributeDef, SecurityKind};
    use std::collections::{BTreeMap, BTreeSet};

    fn fixture() -> (
        SecurityAttributeDef,
        Vec<Security>,
        BTreeMap<String, BTreeMap<String, String>>,
    ) {
        let country = SecurityAttributeDef::new("Country", AttributeKind::Text);
        let securities = vec![
            Security::new("VWCE", "FTSE All-World", "EUR", SecurityKind::Etf),
            Security::new("IWDA", "MSCI World", "USD", SecurityKind::Etf),
            Security::new("SXR8", "S&P 500", "EUR", SecurityKind::Etf),
            Security::new("AAPL", "Apple", "USD", SecurityKind::Stock),
        ];
        let values = securities
            .iter()
            .zip(["Ireland", "Ireland", " Ireland ", "United States"])
            .map(|(s, value)| {
                let one = [(country.id.clone(), value.to_string())].into_iter().collect();
                (s.id.clone(), one)
            })
            .collect();
        (country, securities, values)
    }

    /// Three instruments say Ireland (one of them with stray spaces), one says United States:
    /// two groups, Ireland first because it is the larger one.
    #[test]
    fn distinct_values_become_nodes_ordered_by_size() {
        let (country, securities, values) = fixture();
        let plan = group_by_attribute(&country, &securities, &values, &BTreeSet::new()).unwrap();

        assert_eq!(
            plan.nodes.iter().map(|n| n.path.clone()).collect::<Vec<_>>(),
            [vec!["Ireland".to_string()], vec!["United States".to_string()]]
        );
        assert_eq!(plan.assignments.len(), 4);
        assert_eq!(plan.matched(), 4);
        // Every instrument lands in its group whole: the attribute holds one value, not a split.
        assert!(
            plan.assignments
                .iter()
                .all(|a| a.weight == rust_decimal::Decimal::ONE)
        );
        assert_eq!(plan.name, "Country");
    }

    /// An instrument the tree already places is planned but not assigned: a 70/30 split typed by
    /// hand must survive a regrouping.
    #[test]
    fn an_instrument_the_tree_already_places_is_left_alone() {
        let (country, securities, values) = fixture();
        let classified: BTreeSet<String> = [securities[0].id.clone()].into_iter().collect();
        let plan = group_by_attribute(&country, &securities, &values, &classified).unwrap();

        let kept = plan
            .assignments
            .iter()
            .find(|a| a.symbol == "VWCE")
            .expect("the instrument is still in the plan");
        assert_eq!(kept.security_id, None);
        assert_eq!(kept.matched_by.as_deref(), Some("already_classified"));
        assert_eq!(plan.matched(), 3);
        // Ireland still has two instruments to place, so the node is still planned.
        assert_eq!(plan.nodes.len(), 2);
    }

    /// An instrument without a value is not a group of its own and not an "unknown" node either:
    /// it stays unclassified, which allocation already shows as such.
    #[test]
    fn an_empty_value_makes_no_node() {
        let (country, securities, mut values) = fixture();
        values.insert(
            securities[3].id.clone(),
            [(country.id.clone(), "  ".to_string())].into(),
        );
        let plan = group_by_attribute(&country, &securities, &values, &BTreeSet::new()).unwrap();

        assert_eq!(plan.nodes.len(), 1);
        assert_eq!(plan.assignments.len(), 3);
    }

    #[test]
    fn a_number_is_not_grouped() {
        let ter = SecurityAttributeDef::new("TER", AttributeKind::Number);
        let (_, securities, values) = fixture();
        assert!(group_by_attribute(&ter, &securities, &values, &BTreeSet::new()).is_err());
    }
}
