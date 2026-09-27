use super::*;
use crate::import::{ParseConfig, parse_csv};
use crate::model::SecurityKind;
use rust_decimal_macros::dec;

const PP: &str = "Levels 1,Levels 2,Levels 3,Levels 4,Weight,Allocation,TARGET Value,Symbol,ISIN\n\
Asset Allocation,,,,,100.00,\"2,585.16\",,\n\
Asset Allocation,Global Equities,,,,42.00,\"1,085.77\",,\n\
Asset Allocation,Global Equities,Core,,,88.00,955.48,,\n\
Asset Allocation,Global Equities,Core,Core MSCI World USD (Acc),72.60,,,EUNL.DE,IE00B4L5Y983\n\
Asset Allocation,Bonds,,,,5.00,129.26,,\n\
Asset Allocation,Bonds,Global Aggregate Bond EUR (Acc),,100.00,,,EUNA.DE,IE00BDBRDM35\n\
Without Classification,Apple,,,100.00,,,APC.DE,US0378331005\n";

fn securities() -> Vec<Security> {
    let mut world = Security::new("IWDA.L", "iShares Core MSCI World", "USD", SecurityKind::Etf);
    world.isin = Some("IE00B4L5Y983".into());
    let mut bond = Security::new("EUNA.DE", "Global Aggregate Bond", "EUR", SecurityKind::Etf);
    bond.isin = None;
    vec![world, bond]
}

#[test]
fn portfolio_performance_export_is_read_without_a_single_setting() {
    let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_taxonomy_config(&parsed);

    assert_eq!(config.levels, ["Levels 1", "Levels 2", "Levels 3", "Levels 4"]);
    assert_eq!(config.weight.as_deref(), Some("Weight"));
    assert_eq!(
        config.target.as_deref(),
        Some("Allocation"),
        "\"TARGET Value\" is money, not a target"
    );
    assert!(config.root_is_name, "the first level repeats on every row");

    let securities = securities();
    let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

    assert_eq!(preview.name, "Asset Allocation");

    assert!(!preview.nodes.iter().any(|n| n.path[0].contains("Without")));

    let target_of = |path: &[&str]| {
        preview
            .nodes
            .iter()
            .find(|n| n.path == path.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .and_then(|n| n.target)
    };
    assert_eq!(target_of(&["Global Equities", "Core"]), Some(dec!(0.88)));
    assert_eq!(target_of(&["Global Equities"]), Some(dec!(0.42)));
    assert_eq!(target_of(&["Bonds"]), Some(dec!(0.05)));

    let world = preview
        .assignments
        .iter()
        .find(|a| a.symbol == "EUNL.DE")
        .unwrap();
    assert_eq!(world.path, ["Global Equities", "Core"]);
    assert_eq!(world.weight, dec!(0.726));
    assert_eq!(world.matched_by.as_deref(), Some("isin"));

    let bond = preview
        .assignments
        .iter()
        .find(|a| a.symbol == "EUNA.DE")
        .unwrap();
    assert_eq!(bond.path, ["Bonds"]);
    assert_eq!(bond.matched_by.as_deref(), Some("symbol"));
    assert_eq!(preview.matched(), 2);
}

#[test]
fn targets_are_stored_relative_to_the_parent() {
    let store = Store::open_in_memory().unwrap();
    let portfolio = crate::model::Portfolio::new("Mine", "EUR");
    store.save_portfolio(&portfolio).unwrap();
    let securities = securities();
    for s in &securities {
        store.save_security(s).unwrap();
    }
    let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_taxonomy_config(&parsed);
    let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

    let taxonomy = commit_taxonomy(&store, &preview, None, Some(&portfolio.id)).unwrap();
    let nodes = store.taxonomy_nodes(&taxonomy.id).unwrap();
    let id_of = |name: &str| nodes.iter().find(|n| n.name == name).unwrap().id.clone();
    let target = store
        .targets_for_portfolio(&portfolio.id)
        .unwrap()
        .into_iter()
        .find(|t| t.taxonomy_id == taxonomy.id)
        .unwrap();

    assert_eq!(target.weight_of(&id_of("Global Equities")), Some(dec!(0.42)));
    assert_eq!(target.weight_of(&id_of("Core")), Some(dec!(0.88)));
    let absolute = target.absolute_weights(&nodes);
    assert_eq!(absolute[&id_of("Core")], dec!(0.3696));
    assert_eq!(absolute[&id_of("Bonds")], dec!(0.05));
}

#[test]
fn importing_the_same_file_twice_extends_the_tree_instead_of_doubling_it() {
    let store = Store::open_in_memory().unwrap();
    let securities = securities();
    for s in &securities {
        store.save_security(s).unwrap();
    }
    let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_taxonomy_config(&parsed);
    let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

    let taxonomy = commit_taxonomy(&store, &preview, None, None).unwrap();
    let after_first = store.taxonomy_nodes(&taxonomy.id).unwrap().len();
    assert_eq!(after_first, 3, "Global Equities, Core inside it, and Bonds");

    let before = store.list_taxonomies().unwrap().len();
    commit_taxonomy(&store, &preview, Some(&taxonomy.id), None).unwrap();
    assert_eq!(
        store.list_taxonomies().unwrap().len(),
        before,
        "no second tree was created"
    );
    assert_eq!(store.taxonomy_nodes(&taxonomy.id).unwrap().len(), after_first);

    assert_eq!(store.classifications_for_taxonomy(&taxonomy.id).unwrap().len(), 2);
}

#[test]
fn export_round_trips_through_the_importer() {
    let taxonomy = Taxonomy::new("Regions", TaxonomyKind::Region);
    let us = TaxonomyNode::root(&taxonomy.id, "United States");
    let de = TaxonomyNode::child(&us, "Germany");
    let mut security = Security::new("IWDA.L", "iShares Core MSCI World", "USD", SecurityKind::Etf);
    security.isin = Some("IE00B4L5Y983".into());
    let classifications = vec![SecurityClassification::new(&security.id, &de.id, dec!(0.35))];
    let target = AllocationTarget::new("portfolio", &taxonomy.id, "Target").with_weight(&de.id, dec!(0.2));

    let csv = taxonomy_to_csv(
        &taxonomy,
        &[us.clone(), de.clone()],
        &classifications,
        std::slice::from_ref(&security),
        Some(&target),
    );

    let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
    let config = detect_taxonomy_config(&parsed);
    let preview = build_taxonomy_preview(&parsed, &config, std::slice::from_ref(&security), None);

    assert_eq!(preview.name, "Regions");
    assert_eq!(preview.kind, TaxonomyKind::Region);
    let deepest = preview
        .nodes
        .iter()
        .find(|n| n.path == ["United States", "Germany"])
        .unwrap();
    assert_eq!(deepest.target, Some(dec!(0.2)));
    assert_eq!(preview.assignments.len(), 1);
    assert_eq!(preview.assignments[0].weight, dec!(0.35));
    assert_eq!(preview.assignments[0].path, ["United States", "Germany"]);
}
