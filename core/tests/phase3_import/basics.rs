use super::*;

/// First pass parses columns, dates, and amounts; unknown types remain unresolved.
/// Buy = 1859.95; sell = 955.05; dividend = 2.40; deposit = 5000.
#[test]
fn plain_style_file_is_understood_out_of_the_box() {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);

    let preview = service
        .preview(PLAIN_STYLE.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();

    assert_eq!(preview.config.delimiter, Some(','));
    assert_eq!(preview.config.date_format.as_deref(), Some("%Y-%m-%d"));
    assert_eq!(preview.mapping.column(ImportField::Price), Some("unit_price"));
    // No account is selected, so nothing can be written.
    assert_eq!(preview.summary.ready, 0);

    let mapping = preview.mapping.clone().with_account(&account.id);
    let preview = service
        .preview(
            PLAIN_STYLE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    // SPLIT is a corporate action, not an investor transaction; it stays unresolved.
    assert_eq!(preview.unknown_kinds(), vec!["SPLIT"]);
    assert_eq!(preview.summary.invalid, 1);
    // AAPL is absent: three rows need a security decision; the deposit is ready.
    assert_eq!(preview.summary.unknown_securities, 3);
    assert_eq!(preview.summary.ready, 1);
    assert_eq!(preview.unknown_symbols(), vec!["AAPL"]);

    // The caller names the source: the core ships no default one (ADR-0076), and an import
    // that named none would create instruments nothing ever prices.
    let options = ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    };
    let result = service.commit(&preview, &options).unwrap();
    assert_eq!(result.imported, 4);
    assert_eq!(result.skipped, 1); // SPLIT
    assert_eq!(result.created_securities, vec!["AAPL"]);

    // The named quote source makes the imported security refreshable.
    let created = store.find_security_by_symbol("AAPL").unwrap().unwrap();
    assert_eq!(created.data_source.as_deref(), Some("yahoo"));
    // The provider receives the same symbol that appeared in the file.
    assert_eq!(created.data_symbol, None);
    assert_eq!(created.provider_symbol(), "AAPL");

    // Imported rows follow the normal path; include both depot and cash accounts.
    let transactions = store.transactions_for_accounts(&pair(&account), None).unwrap();
    // USD base and operations need no FX; the lookup source is still explicit.
    let holdings = build_holdings(&transactions, "USD", &store).unwrap();
    let apple = store.find_security_by_symbol("AAPL").unwrap().unwrap();
    let position = &holdings.positions[&apple.id];
    assert_eq!(position.quantity, dec!(5));
    // Cost = 1855.00 + 4.95 = 1859.95; after selling five, 929.975 remains.
    assert_eq!(position.cost_basis, dec!(929.975));
    assert_eq!(holdings.dividends_base, dec!(2.40));
    assert_eq!(
        holdings.cash["USD"],
        dec!(5000) - dec!(1859.95) + dec!(955.05) + dec!(2.40)
    );
}

/// “Manual” means no quote source, not Yahoo by default.
#[test]
fn manual_pricing_leaves_the_created_security_without_a_source() {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    // Reuse the first-pass column mapping, as the wizard does.
    let detected = service
        .preview(PLAIN_STYLE.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();
    let mapping = detected.mapping.clone().with_account(&account.id);
    let preview = service
        .preview(
            PLAIN_STYLE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    let options = ImportOptions {
        new_security_source: None,
        ..ImportOptions::default()
    };
    service.commit(&preview, &options).unwrap();

    let created = store.find_security_by_symbol("AAPL").unwrap().unwrap();
    assert_eq!(created.data_source, None);
}

/// Re-importing the same file does not duplicate history.
#[test]
fn importing_the_same_file_twice_changes_nothing() {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&[
        "date".into(),
        "type".into(),
        "symbol".into(),
        "quantity".into(),
        "unit_price".into(),
        "currency".into(),
        "fee".into(),
        "amount".into(),
        "comment".into(),
    ])
    .with_account(&account.id);

    let first = service
        .preview(
            PLAIN_STYLE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();
    service.commit(&first, &ImportOptions::default()).unwrap();

    let second = service
        .preview(
            PLAIN_STYLE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();
    assert_eq!(second.summary.duplicates, 4);
    assert_eq!(second.summary.ready, 0);

    let result = service.commit(&second, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(
        store
            .transactions_for_accounts(&pair(&account), None)
            .unwrap()
            .len(),
        4,
        "the second import must add nothing"
    );
}

/// A preview is a snapshot: committing the same one twice — a second click on the button —
/// must not write the rows again. 4 rows written, then the same 4 seen in storage, then 0
/// imported and 5 skipped (the unresolved SPLIT row was never importable).
#[test]
fn committing_the_same_preview_twice_writes_nothing_the_second_time() {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&[
        "date".into(),
        "type".into(),
        "symbol".into(),
        "quantity".into(),
        "unit_price".into(),
        "currency".into(),
        "fee".into(),
        "amount".into(),
        "comment".into(),
    ])
    .with_account(&account.id);

    let preview = service
        .preview(
            PLAIN_STYLE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    let first = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(first.imported, 4);

    let again = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(again.imported, 0, "re-committing the same preview writes nothing");
    assert_eq!(again.skipped, 5);
    assert_eq!(
        store
            .transactions_for_accounts(&pair(&account), None)
            .unwrap()
            .len(),
        4,
        "the database still holds the same four transactions"
    );
}

/// German export: semicolons, decimal commas, `dd.mm.yyyy`, and local operation names.
#[test]
fn german_export_needs_one_alias_and_nothing_else() {
    let csv = "\
Datum;Typ;Symbol;Stück;Kurs;Gebühr;Währung
03.06.2024;Kauf;SAP;10;1.234,50;1,90;EUR
10.06.2024;Umbuchung;SAP;5;0;0;EUR
";
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");
    store
        .save_security(&Security::new("SAP", "SAP SE", "EUR", SecurityKind::Stock))
        .unwrap();
    let service = ImportService::new(&store);

    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();
    assert_eq!(preview.config.delimiter, Some(';'));
    assert_eq!(preview.config.decimal_separator, Some(','));
    assert_eq!(preview.config.date_format.as_deref(), Some("%d.%m.%Y"));
    assert_eq!(preview.unknown_kinds(), vec!["Umbuchung"]);

    // One manual mapping resolves the whole file.
    let mapping = preview
        .mapping
        .clone()
        .with_account(&account.id)
        .with_kind_alias("Umbuchung", TransactionKind::DeliveryInbound);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(preview.summary.ready, 2);

    let draft = preview.rows[0].draft.as_ref().unwrap();
    assert_eq!(draft.kind, TransactionKind::Buy);
    assert_eq!(draft.price, dec!(1234.50));
    assert_eq!(draft.amount, dec!(12345.00)); // 10 × 1234.50
    assert_eq!(draft.fees, dec!(1.90));
}

/// Single-cell override: one symbol is wrong, everything else is valid.
#[test]
fn a_single_cell_can_be_corrected_by_hand() {
    let csv = "date,type,symbol,quantity,unit_price,currency\n2024-06-03,BUY,APC.DE,10,100,EUR\n";
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Broker", "EUR");
    let apple = Security::new("AAPL", "Apple Inc.", "EUR", SecurityKind::Stock);
    store.save_security(&apple).unwrap();
    let service = ImportService::new(&store);

    let mapping = ImportMapping::detect(&[
        "date".into(),
        "type".into(),
        "symbol".into(),
        "quantity".into(),
        "unit_price".into(),
        "currency".into(),
    ])
    .with_account(&account.id);

    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(preview.rows[0].status, RowStatus::UnknownSecurity);

    // First option: override one row's cell.
    let overrides = vec![RowOverride::new(1, ImportField::Symbol, "AAPL")];
    let preview = service
        .preview(
            csv.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &overrides,
        )
        .unwrap();
    assert_eq!(preview.rows[0].status, RowStatus::Ready);
    assert_eq!(
        preview.rows[0].draft.as_ref().unwrap().security_id.as_deref(),
        Some(apple.id.as_str())
    );

    // Second option: alias the symbol for the whole file.
    let mapping = mapping.with_symbol_alias("APC.DE", "AAPL");
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(preview.rows[0].status, RowStatus::Ready);
}

/// An unknown security may be skipped instead of created.
#[test]
fn unknown_security_is_skipped_when_asked() {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let csv = "date,type,symbol,quantity,unit_price,currency\n2024-06-03,BUY,TSLA,2,180,USD\n";
    let mapping = ImportMapping::detect(&[
        "date".into(),
        "type".into(),
        "symbol".into(),
        "quantity".into(),
        "unit_price".into(),
        "currency".into(),
    ])
    .with_account(&account.id);

    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    let options = ImportOptions {
        create_missing_securities: false,
        ..ImportOptions::default()
    };
    let result = service.commit(&preview, &options).unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.skipped, 1);
    assert!(store.list_securities().unwrap().is_empty());
}

/// One export can target multiple accounts through an explicit account mapping.
#[test]
fn account_column_routes_rows_to_different_accounts() {
    let store = Store::open_in_memory().unwrap();
    let first = Account::deposit("Deposit", "EUR");
    store.save_account(&first).unwrap();
    let second = depot(&store, "Brokerage", "EUR");

    let csv = "\
date,account,type,amount,currency
2024-06-03,DE-1,DEPOSIT,1000,EUR
2024-06-04,DE-2,DEPOSIT,2000,EUR
2024-06-05,DE-9,DEPOSIT,3000,EUR
";
    let mut mapping = ImportMapping::detect(&[
        "date".into(),
        "account".into(),
        "type".into(),
        "amount".into(),
        "currency".into(),
    ]);
    mapping.account_aliases.insert("DE1".into(), first.id.clone());
    mapping.account_aliases.insert("DE2".into(), second.id.clone());

    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.summary.ready, 2);
    // An unmapped account is a row error, never an arbitrary destination.
    assert_eq!(preview.summary.invalid, 1);

    service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(store.transactions_for_account(&first.id).unwrap().len(), 1);
    // A depot-mapped deposit is stored on its cash account.
    let reference = second.reference_account_id.clone().unwrap();
    assert!(store.transactions_for_account(&second.id).unwrap().is_empty());
    assert_eq!(store.transactions_for_account(&reference).unwrap().len(), 1);
}

/// A cash transfer with a security symbol is likely a security transfer.
/// The importer keeps the symbol and asks for an explicit type.
#[test]
fn cash_transfer_carrying_a_security_is_flagged() {
    let (store, account) = store_with_account();
    store
        .save_security(&Security::new("GOOGL", "Alphabet", "USD", SecurityKind::Stock))
        .unwrap();
    let csv = "date,type,symbol,quantity,unit_price,currency\n2024-05-01,TRANSFER_IN,GOOGL,2,140,USD\n";
    let mapping = ImportMapping::detect(&[
        "date".into(),
        "type".into(),
        "symbol".into(),
        "quantity".into(),
        "unit_price".into(),
        "currency".into(),
    ])
    .with_account(&account.id);

    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.rows[0].status, RowStatus::Invalid);
    assert!(preview.rows[0].problems[0].message.contains("DELIVERY_INBOUND"));
}

/// Price import for one security without a symbol column.
#[test]
fn prices_are_imported_and_found_by_the_lookup() {
    let store = Store::open_in_memory().unwrap();
    let security = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf);
    store.save_security(&security).unwrap();

    let csv = "Datum;Schlusskurs\n03.06.2024;85,10\n04.06.2024;85,90\n05.06.2024;86,40\n";
    let service = ImportService::new(&store);
    let mapping = PriceMapping::detect(&["Datum".into(), "Schlusskurs".into()])
        .with_security(&security.id)
        .with_currency("EUR");

    let import = service
        .preview_prices(csv.as_bytes(), &ParseConfig::default(), Some(&mapping))
        .unwrap();
    assert!(import.problems.is_empty(), "{:?}", import.problems);
    assert_eq!(import.quotes.len(), 3);
    assert_eq!(import.quotes[0].close, dec!(85.10));
    assert_eq!(import.quotes[0].source, "csv");

    assert_eq!(service.commit_prices(&import).unwrap(), 3);

    use sq_core::market::{PriceLookup, PricePoint};
    let sunday = chrono::NaiveDate::from_ymd_opt(2024, 6, 9).unwrap();
    // Imported prices use the same forward-fill as downloaded quotes.
    assert_eq!(
        store.price_as_of(&security.id, sunday).unwrap(),
        Some(PricePoint::new(dec!(86.40), "EUR"))
    );
}
