use super::*;

#[test]
fn ids_are_unique() {
    let mut ids: Vec<&str> = CATALOG.iter().map(|s| s.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), CATALOG.len());
}

#[test]
fn every_constructor_builds_the_source_its_row_names() {
    for source in CATALOG {
        let built: Vec<&str> = [
            source.quotes.map(|make| make(Some("k".into())).id()),
            source.search.map(|make| make(Some("k".into())).id()),
            source.listings.map(|make| make(Some("k".into())).id()),
            source.fx.map(|make| make(Some("k".into())).id()),
            source.index.map(|make| make(Some("k".into())).id()),
        ]
        .into_iter()
        .flatten()
        .collect();
        assert!(!built.is_empty(), "{} plays no role", source.id);
        assert!(
            built.iter().all(|id| *id == source.id),
            "{}: {built:?}",
            source.id
        );
    }
}

#[test]
fn the_defaults_are_on_and_can_answer() {
    let fx = info(DEFAULT_FX).unwrap();
    assert!(fx.on_by_default && fx.capabilities().contains(&Capability::FxRates));
    let index = info(DEFAULT_INDEX).unwrap();
    assert!(index.on_by_default && index.capabilities().contains(&Capability::PriceIndex));
}

/// The point of ADR-0076: a build nobody has answered for asks nobody for a price, and has no
/// provider to stamp a new instrument with.
#[test]
fn no_quote_source_is_on_until_one_is_chosen() {
    // A keyed source is kept out by its missing key; the keyless ones by their own row.
    assert_eq!(default_quotes(&Setup::default()), None);
    assert!(quote_service().provider_ids().is_empty());
}

#[test]
fn services_register_only_what_is_on() {
    let mut setup = Setup::default();
    setup.switched.insert(YahooProvider::ID.into(), true);
    setup.switched.insert(KrakenProvider::ID.into(), true);
    assert_eq!(
        quote_service_with(&setup).provider_ids(),
        vec![KrakenProvider::ID, YahooProvider::ID]
    );
    // The picker's order is the catalogue's, and Yahoo heads it whether or not it is the one
    // a new instrument gets.
    assert_eq!(quote_ids(&setup), vec![YahooProvider::ID, KrakenProvider::ID]);
    assert_eq!(default_quotes(&setup), Some(YahooProvider::ID));
}

#[test]
fn the_central_bank_is_asked_before_the_market() {
    assert_eq!(
        fx_service().provider_ids(),
        vec![EcbProvider::ID, FrankfurterProvider::ID]
    );
    let mut setup = Setup::default();
    setup.switched.insert(YahooProvider::ID.into(), true);
    assert_eq!(
        fx_service_with(&setup).provider_ids(),
        vec![EcbProvider::ID, FrankfurterProvider::ID, YahooProvider::ID]
    );
}

#[test]
fn the_harmonised_index_is_asked_before_the_worldwide_one() {
    assert_eq!(
        index_service().provider_ids(),
        vec![EurostatProvider::ID, ImfProvider::ID]
    );
}

#[test]
fn a_source_that_needs_a_key_is_on_only_once_it_has_one() {
    assert!(
        !quote_ids(&Setup::default())
            .iter()
            .any(|id| id == TwelveDataProvider::ID)
    );
    let mut setup = Setup::default();
    setup.keys.insert(TwelveDataProvider::ID.into(), "k".into());
    assert!(quote_ids(&setup).iter().any(|id| id == TwelveDataProvider::ID));
    assert_eq!(
        fx_service_with(&setup).provider_ids().last(),
        Some(&TwelveDataProvider::ID)
    );
}

#[test]
fn a_custom_source_is_offered_after_the_shipped_ones() {
    let mut setup = Setup::default();
    setup.custom.push(CustomSource {
        id: "custom:bank".into(),
        label: "Bank".into(),
        role: CustomRole::Quotes,
        url: "https://bank.example/{SYMBOL}.json".into(),
        headers: vec![],
        format: crate::market::CustomFormat::Json {
            date_path: "$[*].d".into(),
            close_path: "$[*].c".into(),
        },
        date_format: None,
        factor: None,
        currency: None,
    });
    assert_eq!(quote_ids(&setup).last().map(String::as_str), Some("custom:bank"));
    assert!(quote_service_with(&setup).provider("custom:bank").is_some());

    // The same source as a rate source joins the FX chain last and leaves the quote list.
    setup.custom[0].role = CustomRole::Fx;
    assert_eq!(
        fx_service_with(&setup).provider_ids().last(),
        Some(&"custom:bank")
    );
    assert!(!quote_ids(&setup).iter().any(|id| id == "custom:bank"));
}

#[test]
fn the_user_s_switch_outranks_the_default() {
    let mut setup = Setup::default();
    setup.switched.insert(KrakenProvider::ID.into(), false);
    setup.switched.insert(StooqProvider::ID.into(), true);
    assert_eq!(quote_ids(&setup), vec![StooqProvider::ID]);
    assert_eq!(default_quotes(&setup), Some(StooqProvider::ID));
}
