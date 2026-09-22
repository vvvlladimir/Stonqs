//! Storage round trips and date-based lookup tests.

mod support;

use chrono::Datelike;

use rust_decimal_macros::dec;
use sq_core::calc::PortfolioAnalytics;
use sq_core::fx::{FxRate, RateLookup};
use sq_core::market::{DateRange, PriceLookup, PricePoint, Quote};
use sq_core::model::{
    Account, AccountGroup, Portfolio, Security, SecurityKind, Transaction, TransactionKind,
};
use sq_core::storage::Store;
use support::d;

/// Cash account, depot, and security: the minimum tradeable fixture.
fn seeded() -> (Store, Account, Account, Security) {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("IBKR · cash", "USD");
    store.save_account(&cash).unwrap();
    let depot = Account::securities("Interactive Brokers", "USD", &cash.id);
    let apple =
        Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("stooq", "aapl.us");
    store.save_account(&depot).unwrap();
    store.save_security(&apple).unwrap();
    (store, cash, depot, apple)
}

/// Stored values round-trip without loss, including Decimal precision.
#[test]
fn objects_survive_a_round_trip() {
    let (store, _cash, account, apple) = seeded();

    assert_eq!(store.get_account(&account.id).unwrap(), account);
    assert_eq!(store.get_security(&apple.id).unwrap(), apple);
    assert_eq!(
        store.find_security_by_symbol("AAPL").unwrap().unwrap().id,
        apple.id
    );

    // Text storage preserves 0.1 + 0.2 exactly instead of f64 noise.
    let tx = Transaction::buy(
        &account.id,
        &apple.id,
        d(2024, 6, 5),
        dec!(1.23456789),
        dec!(195.37),
        "USD",
    )
    .with_fees(dec!(0.1))
    .with_fx_rate(dec!(0.923076923076923076923076923));
    store.save_transaction(&tx).unwrap();
    let back = store.transactions_for_account(&account.id).unwrap();
    assert_eq!(back, vec![tx]);
}

/// Portfolio updates replace its account list.
#[test]
fn portfolio_keeps_its_accounts() {
    let (store, _cash, account, _) = seeded();
    let second = Account::deposit("Bank", "EUR");
    store.save_account(&second).unwrap();

    let mut p = Portfolio::new("Main", "EUR").with_accounts([account.id.clone(), second.id.clone()]);
    store.save_portfolio(&p).unwrap();
    assert_eq!(store.get_portfolio(&p.id).unwrap().account_ids.len(), 2);

    p.account_ids = vec![account.id.clone()];
    store.save_portfolio(&p).unwrap();
    assert_eq!(store.get_portfolio(&p.id).unwrap().account_ids, vec![account.id]);
}

/// Date lookup falls back to the latest earlier quote: Friday 07.06.2024 = 196.89;
/// Sunday 09.06 returns it, while dates before the first quote return `None`.
#[test]
fn price_as_of_forward_fills_weekends() {
    let (store, _cash, _depot, apple) = seeded();
    store
        .save_quotes(&[
            Quote {
                security_id: apple.id.clone(),
                date: d(2024, 6, 6),
                close: dec!(194.48),
                currency: "USD".into(),
                source: "stooq".into(),
            },
            Quote {
                security_id: apple.id.clone(),
                date: d(2024, 6, 7),
                close: dec!(196.89),
                currency: "USD".into(),
                source: "stooq".into(),
            },
            Quote {
                security_id: apple.id.clone(),
                date: d(2024, 6, 10),
                close: dec!(193.12),
                currency: "USD".into(),
                source: "stooq".into(),
            },
        ])
        .unwrap();

    assert_eq!(
        store.price_as_of(&apple.id, d(2024, 6, 7)).unwrap(),
        Some(PricePoint::new(dec!(196.89), "USD"))
    );
    assert_eq!(
        store.price_as_of(&apple.id, d(2024, 6, 8)).unwrap(),
        Some(PricePoint::new(dec!(196.89), "USD"))
    );
    assert_eq!(
        store.price_as_of(&apple.id, d(2024, 6, 9)).unwrap(),
        Some(PricePoint::new(dec!(196.89), "USD"))
    );
    assert_eq!(
        store.price_as_of(&apple.id, d(2024, 6, 10)).unwrap(),
        Some(PricePoint::new(dec!(193.12), "USD"))
    );
    // Before the first known quote there is no fallback.
    assert_eq!(store.price_as_of(&apple.id, d(2024, 6, 5)).unwrap(), None);

    assert_eq!(store.latest_quote_date(&apple.id).unwrap(), Some(d(2024, 6, 10)));
    let range = DateRange::new(d(2024, 6, 1), d(2024, 6, 30));
    assert_eq!(store.quotes_in_range(&apple.id, range).unwrap().len(), 3);

    // Previous close steps through quotes, not calendar days.
    assert_eq!(
        store.price_before(&apple.id, d(2024, 6, 10)).unwrap(),
        Some(PricePoint::new(dec!(196.89), "USD"))
    );
    assert_eq!(
        store.price_before(&apple.id, d(2024, 6, 9)).unwrap(),
        Some(PricePoint::new(dec!(194.48), "USD"))
    );
    // The first quote has no previous value.
    assert_eq!(store.price_before(&apple.id, d(2024, 6, 6)).unwrap(), None);
    assert_eq!(store.price_before(&apple.id, d(2024, 6, 5)).unwrap(), None);
}

/// Date FX lookup covers identity, direct, inverse, and fallback pairs.
#[test]
fn fx_rate_as_of_handles_inverse_and_identity() {
    let store = Store::open_in_memory().unwrap();
    store
        .save_fx_rates(&[
            FxRate::new("USD", "EUR", d(2024, 6, 7), dec!(0.92)),
            FxRate::new("USD", "EUR", d(2024, 6, 10), dec!(0.90)),
        ])
        .unwrap();

    assert_eq!(
        store.rate_as_of("EUR", "EUR", d(2024, 6, 9)).unwrap(),
        Some(dec!(1))
    );
    assert_eq!(
        store.rate_as_of("USD", "EUR", d(2024, 6, 9)).unwrap(),
        Some(dec!(0.92))
    );
    assert_eq!(
        store.rate_as_of("USD", "EUR", d(2024, 6, 10)).unwrap(),
        Some(dec!(0.90))
    );

    // Inverse pair is 1 / 0.90; compare with rounding because it repeats.
    let inverse = store.rate_as_of("EUR", "USD", d(2024, 6, 10)).unwrap().unwrap();
    assert_eq!(inverse.round_dp(6), dec!(1.111111));

    assert_eq!(store.rate_as_of("USD", "JPY", d(2024, 6, 10)).unwrap(), None);
    // Missing FX is an error, not an implicit zero.
    assert!(store.convert(dec!(100), "USD", "JPY", d(2024, 6, 10)).is_err());
}

/// End-to-end SQLite run using `calc_examples` arithmetic.
/// 01.06 = 920 EUR; 05.06 cost = 897 EUR; 01.07 value = 967.50 EUR; TWR ≈ +5.163%.
#[test]
fn end_to_end_through_the_database() {
    let (store, cash, account, apple) = seeded();
    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash.id.clone(), account.id.clone()]);
    store.save_portfolio(&portfolio).unwrap();

    store
        .save_transaction(
            &Transaction::cash(
                &cash.id,
                TransactionKind::Deposit,
                d(2024, 6, 1),
                dec!(1000),
                "USD",
            )
            .with_fx_rate(dec!(0.92)),
        )
        .unwrap();
    store
        .save_transaction(
            &Transaction::buy(&account.id, &apple.id, d(2024, 6, 5), dec!(5), dec!(195), "USD")
                .with_fx_rate(dec!(0.92)),
        )
        .unwrap();

    store
        .save_quotes(&[
            Quote {
                security_id: apple.id.clone(),
                date: d(2024, 6, 5),
                close: dec!(195),
                currency: "USD".into(),
                source: "manual".into(),
            },
            Quote {
                security_id: apple.id.clone(),
                date: d(2024, 7, 1),
                close: dec!(210),
                currency: "USD".into(),
                source: "manual".into(),
            },
        ])
        .unwrap();
    store
        .save_fx_rates(&[
            FxRate::new("USD", "EUR", d(2024, 6, 1), dec!(0.92)),
            FxRate::new("USD", "EUR", d(2024, 7, 1), dec!(0.90)),
        ])
        .unwrap();

    let analytics = PortfolioAnalytics::new(&store, &portfolio).unwrap();

    let start = analytics.valuation_at(d(2024, 6, 1)).unwrap();
    assert_eq!(start.total_value_base, dec!(920.00));

    let end = analytics.valuation_at(d(2024, 7, 1)).unwrap();
    assert_eq!(end.securities_value_base, dec!(945.0000));
    assert_eq!(end.cash_base, dec!(22.500));
    assert_eq!(end.total_value_base, dec!(967.5000));
    assert_eq!(end.cost_basis_base, dec!(897.00));
    assert_eq!(end.unrealized_pnl_base, dec!(48.0000));

    let twr = analytics.twr(d(2024, 6, 1), d(2024, 7, 1)).unwrap();
    assert_eq!(twr.round_dp(6), dec!(0.051630));

    // XIRR: −920 EUR on 1 June, +967.50 EUR on 1 July, 30 days.
    let irr = analytics.xirr(d(2024, 7, 1)).unwrap();
    assert_eq!(irr.round_dp(3), dec!(0.845));
}

/// Transactions are returned chronologically and truncated at the date.
#[test]
fn transactions_are_ordered_and_filtered_by_date() {
    let (store, _cash, account, apple) = seeded();
    for day in [20, 5, 12] {
        store
            .save_transaction(&Transaction::buy(
                &account.id,
                &apple.id,
                d(2024, 6, day),
                dec!(1),
                dec!(100),
                "USD",
            ))
            .unwrap();
    }
    let ids = vec![account.id.clone()];
    let all = store.transactions_for_accounts(&ids, None).unwrap();
    assert_eq!(
        all.iter().map(|t| t.date.day()).collect::<Vec<_>>(),
        vec![5, 12, 20]
    );

    let until = store
        .transactions_for_accounts(&ids, Some(d(2024, 6, 12)))
        .unwrap();
    assert_eq!(until.len(), 2);
}

/// Corporate actions and cost-basis method survive a storage round trip.
#[test]
fn corporate_actions_and_cost_basis_round_trip() {
    use sq_core::model::{CorporateAction, CostBasisMethod};

    let (store, _cash, account, apple) = seeded();
    let portfolio = Portfolio::new("Main", "EUR")
        .with_accounts([account.id.clone()])
        .with_cost_basis(CostBasisMethod::AverageCost);
    store.save_portfolio(&portfolio).unwrap();
    assert_eq!(
        store.get_portfolio(&portfolio.id).unwrap().cost_basis_method,
        CostBasisMethod::AverageCost
    );

    let split = CorporateAction::split(&apple.id, d(2024, 6, 1), dec!(1), dec!(4)).with_note("4:1");
    store.save_corporate_action(&split).unwrap();
    assert_eq!(store.list_corporate_actions().unwrap(), vec![split.clone()]);
    assert_eq!(store.corporate_actions_for_security(&apple.id).unwrap().len(), 1);
    assert_eq!(split.quantity_factor().unwrap(), dec!(4));
}

/// Both transfer legs are found through their shared `link_id`.
#[test]
fn linked_transfer_sides_round_trip() {
    let (store, account, _depot, _) = seeded();
    let second = Account::deposit("Second", "EUR");
    store.save_account(&second).unwrap();

    let (out, inc) = Transaction::currency_exchange(
        &account.id,
        &second.id,
        d(2024, 6, 2),
        dec!(1000),
        "USD",
        dec!(920),
        "EUR",
    );
    let link = out.link_id.clone().unwrap();
    store.save_transaction(&out).unwrap();
    store.save_transaction(&inc).unwrap();

    let sides = store.linked_transactions(&link).unwrap();
    assert_eq!(sides.len(), 2);
    assert_eq!(sides.iter().filter(|t| t.currency == "USD").count(), 1);
    assert_eq!(sides.iter().filter(|t| t.currency == "EUR").count(), 1);
}

/// Taxonomy tree persistence covers nodes, weights, and tree queries; ETF allocation is 60/40.
#[test]
fn taxonomy_tree_and_weights_round_trip() {
    use sq_core::model::{SecurityClassification, Taxonomy, TaxonomyKind, TaxonomyNode};

    let (store, _cash, _depot, apple) = seeded();
    let taxonomy = Taxonomy::new("Regions", TaxonomyKind::Region);
    store.save_taxonomy(&taxonomy).unwrap();

    let world = TaxonomyNode::root(&taxonomy.id, "World");
    let us = TaxonomyNode::child(&world, "United States").with_rank(1);
    let eu = TaxonomyNode::child(&world, "Europe").with_rank(2);
    for n in [&world, &us, &eu] {
        store.save_taxonomy_node(n).unwrap();
    }

    let nodes = store.taxonomy_nodes(&taxonomy.id).unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].name, "World", "the root comes first");
    assert_eq!(nodes[1].name, "United States", "then the children by rank");

    store
        .save_classification(&SecurityClassification::new(&apple.id, &us.id, dec!(0.6)))
        .unwrap();
    store
        .save_classification(&SecurityClassification::new(&apple.id, &eu.id, dec!(0.4)))
        .unwrap();

    let cls = store.classifications_for_taxonomy(&taxonomy.id).unwrap();
    assert_eq!(cls.len(), 2);
    assert_eq!(
        cls.iter().map(|c| c.weight).sum::<rust_decimal::Decimal>(),
        dec!(1.0)
    );

    // Weight outside (0, 1] is invalid.
    assert!(
        store
            .save_classification(&SecurityClassification::new(&apple.id, &us.id, dec!(1.5)))
            .is_err()
    );
}

/// Deleting a node cascades to its allocations.
#[test]
fn deleting_a_node_removes_its_classifications() {
    use sq_core::model::{SecurityClassification, Taxonomy, TaxonomyKind, TaxonomyNode};

    let (store, _cash, _depot, apple) = seeded();
    let taxonomy = Taxonomy::new("Asset classes", TaxonomyKind::AssetClass);
    store.save_taxonomy(&taxonomy).unwrap();
    let node = TaxonomyNode::root(&taxonomy.id, "Equities");
    store.save_taxonomy_node(&node).unwrap();
    store
        .save_classification(&SecurityClassification::new(&apple.id, &node.id, dec!(1)))
        .unwrap();

    store.delete_taxonomy_node(&node.id).unwrap();
    assert!(
        store
            .classifications_for_taxonomy(&taxonomy.id)
            .unwrap()
            .is_empty()
    );
}

/// A security can be deleted until a transaction references it.
#[test]
fn a_security_used_by_a_transaction_cannot_be_deleted() {
    let (store, _cash, account, apple) = seeded();
    let day = d(2024, 6, 5);

    // No transactions: deletion succeeds.
    let unused = Security::new("MSFT", "Microsoft", "USD", SecurityKind::Stock);
    store.save_security(&unused).unwrap();
    store.delete_security(&unused.id).unwrap();
    assert!(store.get_security(&unused.id).is_err());

    // Once referenced, deletion is rejected with a useful error.
    let buy = Transaction::buy(&account.id, &apple.id, day, dec!(1), dec!(195), "USD");
    store.save_transaction(&buy).unwrap();
    let err = store.delete_security(&apple.id).unwrap_err().to_string();
    assert!(
        err.contains("used by 1 transactions"),
        "uninformative error: {err}"
    );
    assert!(store.get_security(&apple.id).is_ok());
}

/// File-backed databases open in WAL mode; in-memory SQLite ignores this PRAGMA.
/// WAL allows reads while quote writes are in progress.
#[test]
fn a_file_database_runs_in_wal_mode() {
    let path = std::env::temp_dir().join(format!("portfolio-wal-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);

    {
        let store = Store::open(&path).unwrap();
        let mode: String = store
            .conn()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");

        let timeout: i64 = store
            .conn()
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(timeout, 5000);
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
}

/// Currencies used only by transactions remain in the currency list.
/// A CHF fee must produce a CHF prefetch pair or valuation hits `MissingMarketData`.
#[test]
fn currency_seen_only_in_a_transaction_is_still_listed() {
    let (store, cash, _depot, _apple) = seeded();
    // Account and security use USD; the fee uses CHF.
    store
        .save_transaction(&Transaction::cash(
            &cash.id,
            TransactionKind::Fee,
            d(2024, 6, 1),
            dec!(12),
            "CHF",
        ))
        .unwrap();

    let currencies = store.distinct_currencies().unwrap();
    assert_eq!(
        currencies,
        ["CHF".to_string(), "USD".to_string()].into_iter().collect()
    );
}

/// A depot points to its cash account, never to another depot.
#[test]
fn a_depot_points_at_a_deposit_account() {
    let (store, cash, depot, _) = seeded();

    assert_eq!(
        store.get_account(&depot.id).unwrap().reference_account_id,
        Some(cash.id.clone())
    );
    assert_eq!(store.accounts_referencing(&cash.id).unwrap(), vec![depot.clone()]);

    let nested = Account::securities("Nested depot", "USD", &depot.id);
    let err = store.save_account(&nested).unwrap_err().to_string();
    assert!(err.contains("not a cash account"), "uninformative error: {err}");
}

/// Account kind determines valid transactions; storage validates this because transactions carry only IDs.
#[test]
fn transactions_land_on_the_right_kind_of_account() {
    let (store, cash, depot, apple) = seeded();
    let day = d(2024, 6, 5);

    // A cash account cannot hold a security purchase.
    let wrong_buy = Transaction::buy(&cash.id, &apple.id, day, dec!(1), dec!(195), "USD");
    let err = store.save_transaction(&wrong_buy).unwrap_err().to_string();
    assert!(err.contains("securities account"), "uninformative error: {err}");

    // A depot cannot receive cash deposits.
    let wrong_cash = Transaction::cash(&depot.id, TransactionKind::Deposit, day, dec!(100), "USD");
    assert!(store.save_transaction(&wrong_cash).is_err());

    // The matching valid cases are accepted.
    store
        .save_transaction(&Transaction::buy(
            &depot.id,
            &apple.id,
            day,
            dec!(1),
            dec!(195),
            "USD",
        ))
        .unwrap();
    store
        .save_transaction(&Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            day,
            dec!(100),
            "USD",
        ))
        .unwrap();
}

/// Account groups store a replaceable membership snapshot; deleting a group keeps accounts.
#[test]
fn account_groups_round_trip() {
    let (store, cash, depot, _) = seeded();

    let group = AccountGroup::new("Pension").with_accounts([cash.id.clone(), depot.id.clone()]);
    store.save_account_group(&group).unwrap();
    assert_eq!(store.get_account_group(&group.id).unwrap().account_ids.len(), 2);

    let narrowed = AccountGroup {
        account_ids: vec![depot.id.clone()],
        ..group.clone()
    };
    store.save_account_group(&narrowed).unwrap();
    assert_eq!(
        store.get_account_group(&group.id).unwrap().account_ids,
        vec![depot.id.clone()]
    );

    store.delete_account_group(&group.id).unwrap();
    assert!(store.list_account_groups().unwrap().is_empty());
    assert!(store.get_account(&depot.id).is_ok());
}

/// Deleting an account also removes it from group membership.
#[test]
fn deleting_an_account_removes_it_from_groups() {
    let (store, cash, depot, _) = seeded();
    let group = AccountGroup::new("Everything").with_accounts([cash.id.clone(), depot.id.clone()]);
    store.save_account_group(&group).unwrap();

    store.delete_account(&depot.id).unwrap();
    assert_eq!(
        store.get_account_group(&group.id).unwrap().account_ids,
        vec![cash.id]
    );
}

/// Listing venue survives a security round trip.
/// A symbol is not enough: local tickers vary by country (`EUNL`/`SWDA`).
#[test]
fn a_securitys_listing_venue_round_trips() {
    let (store, _, _, security) = seeded();

    // A manually created security has no selected listing, not an unknown one.
    assert_eq!(store.get_security(&security.id).unwrap().mic, None);

    let listed = Security {
        symbol: "EUNL.DE".into(),
        mic: Some("XETR".into()),
        ..security.clone()
    };
    store.save_security(&listed).unwrap();

    let read = store.get_security(&security.id).unwrap();
    assert_eq!(read.mic.as_deref(), Some("XETR"));
    assert_eq!(
        sq_core::market::mic::market_name("XETR"),
        Some("Xetra"),
        "the venue name is derived from the code, not stored as a second field"
    );
}

/// Requested coverage and received quotes are separate results: requested 1–10 June,
/// received quotes on 3, 5, and 7 June.
#[test]
fn quote_stats_report_what_actually_arrived() {
    let (store, _, _, security) = seeded();

    store
        .extend_quote_coverage(&security.id, DateRange::new(d(2024, 6, 1), d(2024, 6, 10)))
        .unwrap();
    store
        .save_quotes(&[
            quote(&security.id, d(2024, 6, 3), dec!(100)),
            quote(&security.id, d(2024, 6, 5), dec!(110)),
            quote(&security.id, d(2024, 6, 7), dec!(105)),
        ])
        .unwrap();

    let stats = store.quote_stats().unwrap();
    let stat = &stats[&security.id];

    assert_eq!(stat.count, 3);
    assert_eq!(stat.range.from, d(2024, 6, 3));
    assert_eq!(stat.range.to, d(2024, 6, 7));
    assert_eq!(stat.currency, "USD", "the currency comes from the latest quote");

    // Requested coverage starts on the 1st; the first quote is on the 3rd.
    let coverage = store.quote_coverage(&security.id).unwrap().unwrap();
    assert_eq!(coverage.from, d(2024, 6, 1));

    // A security with no quotes is omitted; the table, not the query, may show zero.
    let other = Security::new("MSFT", "Microsoft", "USD", SecurityKind::Stock);
    store.save_security(&other).unwrap();
    assert!(!store.quote_stats().unwrap().contains_key(&other.id));
}

fn quote(security_id: &str, date: chrono::NaiveDate, close: rust_decimal::Decimal) -> Quote {
    Quote {
        security_id: security_id.to_string(),
        date,
        close,
        currency: "USD".into(),
        source: "yahoo".into(),
    }
}

/// A charge billed in its own currency survives the database; one billed in the transaction's
/// own currency is stored as absent, so a row has a single spelling of "the same currency".
#[test]
fn a_charge_keeps_the_currency_it_was_billed_in() {
    let (store, _cash, depot, apple) = seeded();
    let foreign = Transaction::buy(&depot.id, &apple.id, d(2024, 3, 1), dec!(10), dec!(100), "USD")
        .with_fees_in(dec!(12), "EUR")
        .with_taxes_in(dec!(3), "usd");
    store.save_transaction(&foreign).unwrap();

    let read = store.transactions_for_account(&depot.id).unwrap();
    assert_eq!(read[0].fee_currency.as_deref(), Some("EUR"));
    assert_eq!(read[0].fees_in(), "EUR");
    assert_eq!(
        read[0].tax_currency, None,
        "the transaction's own currency is not repeated"
    );
    assert_eq!(read[0].taxes_in(), "USD");
}
