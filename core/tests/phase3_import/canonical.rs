use super::*;

/// A portfolio written out and read back into an empty one: same operations, same identity.
/// Buy 10 × 100 EUR with a 12 USD commission, a dividend of 40 EUR taxed 6 EUR, a deposit.
#[test]
fn a_portfolio_exported_and_imported_again_is_the_same_portfolio() {
    let (store, depot) = store_with_account();
    let cash = store
        .list_accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.id != depot.id)
        .expect("the depot has a cash account");
    let apple = Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock);
    store.save_security(&apple).unwrap();

    let written = vec![
        Transaction::buy(
            &depot.id,
            &apple.id,
            "2024-03-01".parse().unwrap(),
            dec!(10),
            dec!(100),
            "USD",
        )
        .with_fees_in(dec!(12), "EUR")
        .with_external_id("TR-1"),
        Transaction::dividend(
            &depot.id,
            &apple.id,
            "2024-06-03".parse().unwrap(),
            dec!(40),
            "USD",
        )
        .with_taxes(dec!(6)),
        Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            "2024-01-02".parse().unwrap(),
            dec!(5000),
            "USD",
        ),
    ];
    for t in &written {
        store.save_transaction(t).unwrap();
    }

    let file = canonical_to_file(
        &written,
        &store.list_accounts().unwrap(),
        &store.list_securities().unwrap(),
    )
    .unwrap();

    // A fresh portfolio with the same account names and nothing else in it.
    let other = Store::open_in_memory().unwrap();
    let other_cash = Account::deposit(&cash.name, "USD");
    other.save_account(&other_cash).unwrap();
    let other_depot = Account::securities(&depot.name, "USD", &other_cash.id);
    other.save_account(&other_depot).unwrap();

    let service = ImportService::new(&other);
    let preview = service
        .preview(file.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();

    // The file names its accounts and its own operation wordings, so nothing is left to answer.
    assert_eq!(preview.summary.invalid, 0);
    assert_eq!(preview.summary.total, 3);
    assert!(preview.unmapped_accounts().is_empty());

    let result = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 3);

    // Same rows, down to what makes them identical for a later import.
    let mut before: Vec<String> = written.iter().map(sq_core::import::fingerprint_of).collect();
    let mut after: Vec<String> = other
        .list_accounts()
        .unwrap()
        .iter()
        .flat_map(|a| other.transactions_for_account(&a.id).unwrap())
        .map(|t| sq_core::import::fingerprint_of(&t))
        .collect();
    // Account and instrument ids belong to the portfolio that holds them, not to the file, so
    // identity is compared on what the file actually carried: the operation itself.
    let strip = |prints: &mut Vec<String>| {
        for print in prints.iter_mut() {
            let parts: Vec<&str> = print.split('|').collect();
            *print = [parts[1], parts[2], parts[4], parts[5], parts[6]].join("|");
        }
        prints.sort();
    };
    strip(&mut before);
    strip(&mut after);
    assert_eq!(before, after);

    // The charge kept the currency it was billed in, and the broker's own name survived.
    let rows = other.transactions_for_account(&other_depot.id).unwrap();
    let buy = rows.iter().find(|t| t.kind == TransactionKind::Buy).unwrap();
    assert_eq!(buy.fees, dec!(12));
    assert_eq!(buy.fee_currency.as_deref(), Some("EUR"));
    assert_eq!(buy.external_id.as_deref(), Some("TR-1"));

    // Re-importing the same file writes nothing: the identifiers are already there.
    let again = ImportService::new(&other)
        .preview(file.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();
    assert_eq!(again.summary.duplicates, 3);
}
