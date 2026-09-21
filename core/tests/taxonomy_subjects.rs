//! Cash as a taxonomy subject, plus subject exclusion.

mod support;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::{PortfolioAnalytics, SubjectKind, UNCLASSIFIED_KEY};
use sq_core::market::Quote;
use sq_core::model::{
    Account, CashClassification, Portfolio, Security, SecurityClassification, SecurityKind, Taxonomy,
    TaxonomyKind, TaxonomyNode, Transaction, TransactionKind, cash_subject_key,
};
use sq_core::storage::Store;
use support::d;

/// Cash account, depot, security, and two-category taxonomy: 02.01 +1000;
/// 03.01 10 × 90 + 1 = 901; 31.12 10 × 100 = 1000; cash = 99; total = 1099 EUR.
struct World {
    store: Store,
    portfolio: Portfolio,
    cash_account: Account,
    etf: Security,
    taxonomy: Taxonomy,
    equities: TaxonomyNode,
    buffer: TaxonomyNode,
}

fn seeded() -> World {
    let store = Store::open_in_memory().unwrap();

    let cash_account = Account::deposit("Savings", "EUR");
    store.save_account(&cash_account).unwrap();
    let depot = Account::securities("TR", "EUR", &cash_account.id);
    store.save_account(&depot).unwrap();

    let etf = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf);
    store.save_security(&etf).unwrap();
    store
        .save_quotes(&[Quote {
            security_id: etf.id.clone(),
            date: d(2024, 12, 31),
            close: dec!(100),
            currency: "EUR".into(),
            source: "manual".into(),
        }])
        .unwrap();

    for t in [
        Transaction::cash(
            &cash_account.id,
            TransactionKind::Deposit,
            d(2024, 1, 2),
            dec!(1000),
            "EUR",
        ),
        Transaction::buy(&depot.id, &etf.id, d(2024, 1, 3), dec!(10), dec!(90), "EUR").with_fees(dec!(1)),
    ] {
        store.save_transaction(&t).unwrap();
    }

    let taxonomy = Taxonomy::new("Asset class", TaxonomyKind::AssetClass);
    store.save_taxonomy(&taxonomy).unwrap();
    let equities = TaxonomyNode::root(&taxonomy.id, "Equities");
    let buffer = TaxonomyNode::root(&taxonomy.id, "Buffer");
    store.save_taxonomy_node(&equities).unwrap();
    store.save_taxonomy_node(&buffer).unwrap();

    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash_account.id.clone(), depot.id.clone()]);
    World {
        store,
        portfolio,
        cash_account,
        etf,
        taxonomy,
        equities,
        buffer,
    }
}

/// Cash receives a category like a security.
/// Security = 1000; cash = 99; total = 1099; stocks = 0.909918107…; reserve = 0.090081892….
#[test]
fn cash_of_an_account_is_classified_like_a_security() {
    let w = seeded();
    w.store
        .save_classification(&SecurityClassification::new(
            &w.etf.id,
            &w.equities.id,
            Decimal::ONE,
        ))
        .unwrap();
    w.store
        .save_cash_classification(&CashClassification::new(
            &w.cash_account.id,
            "EUR",
            &w.buffer.id,
            Decimal::ONE,
        ))
        .unwrap();

    let analytics = PortfolioAnalytics::new(&w.store, &w.portfolio).unwrap();
    let allocation = analytics
        .allocation_by_taxonomy(&w.taxonomy.id, d(2024, 12, 31))
        .unwrap();

    assert_eq!(allocation.total_base, dec!(1099));
    assert_eq!(allocation.find(&w.equities.id).unwrap().value_base, dec!(1000));
    assert_eq!(allocation.find(&w.buffer.id).unwrap().value_base, dec!(99));
    assert!(
        allocation.find(UNCLASSIFIED_KEY).is_none(),
        "everything is classified"
    );
    assert_eq!(allocation.total_weight().round_dp(10), Decimal::ONE);

    // Cash is a normal row keyed by account and currency, with `kind` identifying it.
    let members = analytics
        .allocation_members(&w.taxonomy.id, None, d(2024, 12, 31))
        .unwrap();
    let cash = members
        .iter()
        .find(|m| m.kind == SubjectKind::Cash)
        .expect("cash is a member");
    assert_eq!(cash.subject_id, cash_subject_key(&w.cash_account.id, "EUR"));
    assert_eq!(cash.symbol, "EUR");
    assert_eq!(cash.name, "Savings");
    assert_eq!(cash.value_base, dec!(99));
}

/// An excluded subject leaves calculations but remains in storage.
/// Excluding it makes the denominator 1099 − 1000 = 99 and reserve = 100%; re-enable restores stocks.
#[test]
fn a_disabled_subject_leaves_the_maths_but_keeps_its_classification() {
    let w = seeded();
    w.store
        .save_classification(&SecurityClassification::new(
            &w.etf.id,
            &w.equities.id,
            Decimal::ONE,
        ))
        .unwrap();
    w.store
        .save_cash_classification(&CashClassification::new(
            &w.cash_account.id,
            "EUR",
            &w.buffer.id,
            Decimal::ONE,
        ))
        .unwrap();
    w.store
        .set_taxonomy_exclusion(&w.taxonomy.id, &w.etf.id, true)
        .unwrap();

    let analytics = PortfolioAnalytics::new(&w.store, &w.portfolio).unwrap();
    let allocation = analytics
        .allocation_by_taxonomy(&w.taxonomy.id, d(2024, 12, 31))
        .unwrap();

    assert_eq!(allocation.total_base, dec!(99));
    assert_eq!(allocation.find(&w.equities.id).unwrap().value_base, Decimal::ZERO);
    assert_eq!(allocation.find(&w.buffer.id).unwrap().weight, Decimal::ONE);
    assert!(allocation.find(UNCLASSIFIED_KEY).is_none());

    // It remains listed with zero weight and an exclusion flag.
    let members = analytics
        .allocation_members(&w.taxonomy.id, None, d(2024, 12, 31))
        .unwrap();
    assert_eq!(members.len(), 2);
    let last = members.last().unwrap();
    assert_eq!(last.subject_id, w.etf.id);
    assert!(last.excluded);
    assert_eq!(last.weight, Decimal::ZERO);
    assert_eq!(last.subject_value_base, dec!(1000), "the value did not disappear");

    // The allocation is disabled, not deleted.
    assert_eq!(
        w.store
            .classifications_for_taxonomy(&w.taxonomy.id)
            .unwrap()
            .len(),
        1
    );
    w.store
        .set_taxonomy_exclusion(&w.taxonomy.id, &w.etf.id, false)
        .unwrap();
    let allocation = analytics
        .allocation_by_taxonomy(&w.taxonomy.id, d(2024, 12, 31))
        .unwrap();
    assert_eq!(allocation.find(&w.equities.id).unwrap().value_base, dec!(1000));
}

/// Exclusion belongs to the taxonomy/subject pair, not the security.
/// A security excluded from asset class remains active in the region taxonomy.
#[test]
fn disabling_is_scoped_to_one_tree() {
    let w = seeded();
    let regions = Taxonomy::new("Region", TaxonomyKind::Region);
    w.store.save_taxonomy(&regions).unwrap();
    let world = TaxonomyNode::root(&regions.id, "World");
    w.store.save_taxonomy_node(&world).unwrap();
    w.store
        .save_classification(&SecurityClassification::new(&w.etf.id, &world.id, Decimal::ONE))
        .unwrap();
    w.store
        .set_taxonomy_exclusion(&w.taxonomy.id, &w.etf.id, true)
        .unwrap();

    let analytics = PortfolioAnalytics::new(&w.store, &w.portfolio).unwrap();
    let by_region = analytics
        .allocation_by_taxonomy(&regions.id, d(2024, 12, 31))
        .unwrap();

    assert_eq!(
        by_region.total_base,
        dec!(1099),
        "in this tree the security is classified"
    );
    assert_eq!(by_region.find(&world.id).unwrap().value_base, dec!(1000));
    assert_eq!(w.store.taxonomy_exclusions(&regions.id).unwrap().len(), 0);
    assert_eq!(w.store.taxonomy_exclusions(&w.taxonomy.id).unwrap(), [w.etf.id]);
}
