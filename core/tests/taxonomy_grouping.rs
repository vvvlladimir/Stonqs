//! Building a taxonomy out of an instrument attribute, and what a second run does.

use rust_decimal_macros::dec;
use sq_core::import::{commit_taxonomy, group_by_attribute};
use sq_core::model::{
    AttributeKind, Security, SecurityAttributeDef, SecurityClassification, SecurityKind, Taxonomy,
    TaxonomyKind,
};
use sq_core::storage::Store;
use std::collections::BTreeSet;

struct World {
    store: Store,
    country: SecurityAttributeDef,
    securities: Vec<Security>,
}

/// Three ETFs domiciled in Ireland, one US stock, and one instrument whose country nobody filled in.
fn seeded() -> World {
    let store = Store::open_in_memory().unwrap();
    let country = SecurityAttributeDef::new("Country", AttributeKind::Text);
    store.save_attribute_def(&country).unwrap();

    let securities = vec![
        Security::new("VWCE", "FTSE All-World", "EUR", SecurityKind::Etf),
        Security::new("IWDA", "MSCI World", "USD", SecurityKind::Etf),
        Security::new("SXR8", "S&P 500", "EUR", SecurityKind::Etf),
        Security::new("AAPL", "Apple", "USD", SecurityKind::Stock),
        Security::new("GOLD", "Physical gold", "EUR", SecurityKind::Other),
    ];
    for (security, value) in securities.iter().zip([
        Some("Ireland"),
        Some("Ireland"),
        Some("Ireland"),
        Some("United States"),
        None,
    ]) {
        store.save_security(security).unwrap();
        if let Some(value) = value {
            let values = [(country.id.clone(), value.to_string())].into_iter().collect();
            store.set_security_attributes(&security.id, &values).unwrap();
        }
    }

    World {
        store,
        country,
        securities,
    }
}

fn plan_into(world: &World, taxonomy: Option<&Taxonomy>) -> sq_core::import::TaxonomyPreview {
    let classified: BTreeSet<String> = match taxonomy {
        Some(t) => world
            .store
            .classifications_for_taxonomy(&t.id)
            .unwrap()
            .into_iter()
            .map(|c| c.security_id)
            .collect(),
        None => BTreeSet::new(),
    };
    group_by_attribute(
        &world.country,
        &world.store.list_securities().unwrap(),
        &world.store.security_attributes().unwrap(),
        &classified,
    )
    .unwrap()
}

/// 3 instruments say Ireland, 1 says United States, 1 says nothing: two categories, four
/// classifications, and the fifth instrument stays where it was — unclassified.
#[test]
fn a_tree_is_built_from_the_values_of_one_attribute() {
    let world = seeded();
    let taxonomy = Taxonomy::new("Country", TaxonomyKind::Custom);
    world.store.save_taxonomy(&taxonomy).unwrap();

    let plan = plan_into(&world, Some(&taxonomy));
    commit_taxonomy(&world.store, &plan, Some(&taxonomy.id), None).unwrap();

    let nodes = world.store.taxonomy_nodes(&taxonomy.id).unwrap();
    let mut names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["Ireland", "United States"]);
    // Largest group first: the node order is what the charts colour by.
    assert_eq!(nodes.iter().min_by_key(|n| n.rank).unwrap().name, "Ireland");

    let classifications = world.store.classifications_for_taxonomy(&taxonomy.id).unwrap();
    assert_eq!(classifications.len(), 4);
    // One value, one node, the whole instrument: 4 × 1.0.
    assert!(classifications.iter().all(|c| c.weight == dec!(1)));
    let gold = &world.securities[4];
    assert!(!classifications.iter().any(|c| c.security_id == gold.id));
}

/// A split typed by hand outranks the column: grouping again neither moves VWCE nor duplicates
/// the categories it created the first time.
#[test]
fn a_second_run_leaves_a_hand_made_split_alone() {
    let world = seeded();
    let taxonomy = Taxonomy::new("Country", TaxonomyKind::Custom);
    world.store.save_taxonomy(&taxonomy).unwrap();
    commit_taxonomy(
        &world.store,
        &plan_into(&world, Some(&taxonomy)),
        Some(&taxonomy.id),
        None,
    )
    .unwrap();

    // The user splits VWCE 60/40 across the two categories by hand.
    let nodes = world.store.taxonomy_nodes(&taxonomy.id).unwrap();
    let ireland = nodes.iter().find(|n| n.name == "Ireland").unwrap();
    let states = nodes.iter().find(|n| n.name == "United States").unwrap();
    let vwce = &world.securities[0];
    world
        .store
        .save_classification(&SecurityClassification::new(&vwce.id, &ireland.id, dec!(0.6)))
        .unwrap();
    world
        .store
        .save_classification(&SecurityClassification::new(&vwce.id, &states.id, dec!(0.4)))
        .unwrap();

    let again = plan_into(&world, Some(&taxonomy));
    // Every instrument the tree already places is planned but not assigned.
    assert_eq!(again.matched(), 0);
    assert_eq!(again.nodes.len(), 0);
    commit_taxonomy(&world.store, &again, Some(&taxonomy.id), None).unwrap();

    assert_eq!(world.store.taxonomy_nodes(&taxonomy.id).unwrap().len(), 2);
    let mine: Vec<_> = world
        .store
        .classifications_for_taxonomy(&taxonomy.id)
        .unwrap()
        .into_iter()
        .filter(|c| c.security_id == vwce.id)
        .collect();
    assert_eq!(mine.len(), 2);
    assert!(
        mine.iter()
            .any(|c| c.node_id == states.id && c.weight == dec!(0.4))
    );
}

/// An instrument whose value is filled in after the first run joins the tree on the second,
/// reusing the category rather than creating a second one of the same name.
#[test]
fn a_newly_filled_value_joins_an_existing_category() {
    let world = seeded();
    let taxonomy = Taxonomy::new("Country", TaxonomyKind::Custom);
    world.store.save_taxonomy(&taxonomy).unwrap();
    commit_taxonomy(
        &world.store,
        &plan_into(&world, Some(&taxonomy)),
        Some(&taxonomy.id),
        None,
    )
    .unwrap();

    let gold = &world.securities[4];
    let values = [(world.country.id.clone(), "Ireland".to_string())]
        .into_iter()
        .collect();
    world.store.set_security_attributes(&gold.id, &values).unwrap();

    let again = plan_into(&world, Some(&taxonomy));
    assert_eq!(again.matched(), 1);
    commit_taxonomy(&world.store, &again, Some(&taxonomy.id), None).unwrap();

    assert_eq!(world.store.taxonomy_nodes(&taxonomy.id).unwrap().len(), 2);
    let classifications = world.store.classifications_for_taxonomy(&taxonomy.id).unwrap();
    assert_eq!(classifications.len(), 5);
    let ireland = world
        .store
        .taxonomy_nodes(&taxonomy.id)
        .unwrap()
        .into_iter()
        .find(|n| n.name == "Ireland")
        .unwrap();
    assert!(
        classifications
            .iter()
            .any(|c| c.security_id == gold.id && c.node_id == ireland.id)
    );
}
