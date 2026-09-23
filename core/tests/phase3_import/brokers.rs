use super::*;

/// Real Trade Republic headers with one row of each operation type — the conformance fixture,
/// read from disk so the sample these tests reason about is the one the expectation was
/// generated from. Edge cases: ISIN in `symbol`, negative sales, `DEFAULT` account,
/// source-currency `fx_rate`.
fn trade_republic() -> String {
    conformance::sample("trade-republic", "export.csv")
}

fn trade_republic_mapping(account: &Account) -> ImportMapping {
    let file = trade_republic();
    let headers: Vec<String> = file
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    ImportMapping::detect(&headers).with_account(&account.id)
}

/// The file parses without aliases; only account selection remains.
/// Three rows need missing securities, a separate status rather than an error.
#[test]
fn trade_republic_export_needs_only_the_account() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");

    let service = ImportService::new(&store);
    let mapping = trade_republic_mapping(&account);
    let preview = service
        .preview(
            trade_republic().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    // Instrument name has its own column; otherwise the ISIN would be the name.
    assert_eq!(mapping.column(ImportField::Name), Some("name"));
    assert_eq!(preview.summary.invalid, 0, "{:?}", preview.rows);
    assert_eq!(preview.summary.ready, 1, "a cash row needs no security");
    assert_eq!(preview.summary.unknown_securities, 3);

    // Sale quantity has broker sign; operation kind carries direction.
    let sell = &preview.rows[2].draft.as_ref().unwrap();
    assert_eq!(sell.kind, TransactionKind::Sell);
    assert_eq!(sell.quantity, dec!(0.361938));

    // ISIN is recognized directly from `symbol`; there is no separate ISIN column.
    let etf = preview
        .symbols
        .iter()
        .find(|s| s.value == "IE00B5BMR087")
        .unwrap();
    assert_eq!(etf.isin.as_deref(), Some("IE00B5BMR087"));
    assert_eq!(etf.file_name.as_deref(), Some("Core S&P 500 USD (Acc)"));
    assert_eq!(etf.count, 2);
    assert!(etf.required);
}

/// A resolved security uses its ticker, name, currency, and quote source.
#[test]
fn a_resolved_isin_becomes_a_security_with_a_real_ticker() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");

    // This is the mapping produced after provider lookup.
    let mapping = trade_republic_mapping(&account).with_new_security(
        "IE00B5BMR087",
        SecurityDraft {
            symbol: "CSSPX.MI".into(),
            name: "iShares Core S&P 500 UCITS ETF USD (Acc)".into(),
            currency: "EUR".into(),
            kind: SecurityKind::Etf,
            isin: Some("IE00B5BMR087".into()),
            data_source: Some("yahoo".into()),
            data_symbol: None,
            exchange: Some("Milan".into()),
            mic: Some("XMIL".into()),
        },
    );

    let service = ImportService::new(&store);
    let preview = service
        .preview(
            trade_republic().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    // The preview shows the resolved security before it is stored.
    let etf = preview
        .symbols
        .iter()
        .find(|s| s.value == "IE00B5BMR087")
        .unwrap();
    assert_eq!(
        etf.name.as_deref(),
        Some("iShares Core S&P 500 UCITS ETF USD (Acc)")
    );
    assert_eq!(preview.unresolved_symbols().len(), 1, "only Apple is left");

    service.commit(&preview, &ImportOptions::default()).unwrap();

    let created = store.find_security_by_symbol("CSSPX.MI").unwrap().unwrap();
    assert_eq!(created.name, "iShares Core S&P 500 UCITS ETF USD (Acc)");
    assert_eq!(created.currency, "EUR");
    assert_eq!(created.kind, SecurityKind::Etf);
    assert_eq!(created.isin.as_deref(), Some("IE00B5BMR087"));
    assert_eq!(created.data_source.as_deref(), Some("yahoo"));

    // Both rows reference the same security record.
    let linked = store
        .transactions_for_account(&account.id)
        .unwrap()
        .into_iter()
        .filter(|t| t.security_id.as_deref() == Some(created.id.as_str()))
        .count();
    assert_eq!(linked, 2);
}

/// An unresolved ISIN imports without a quote source and with an explicit warning.
#[test]
fn an_unresolved_isin_is_created_without_a_quote_source() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");

    let service = ImportService::new(&store);
    let mapping = trade_republic_mapping(&account);
    let preview = service
        .preview(
            trade_republic().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();
    let result = service.commit(&preview, &ImportOptions::default()).unwrap();

    let created = store.find_security_by_symbol("IE00B5BMR087").unwrap().unwrap();
    assert!(created.data_source.is_none());
    assert_eq!(created.isin.as_deref(), Some("IE00B5BMR087"));
    // Keep the file name; a bare ISIN is unreadable in the positions list.
    assert_eq!(created.name, "Core S&P 500 USD (Acc)");
    assert!(
        result
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::SecurityWithoutSource),
        "{:?}",
        result.problems
    );
}

/// `fx_rate` does not alter base-currency amounts; dividend = 0.22 − 0.03 = 0.19 EUR.
#[test]
fn the_brokers_fx_column_does_not_shrink_amounts_in_the_base_currency() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");

    let service = ImportService::new(&store);
    let mapping = trade_republic_mapping(&account);
    let preview = service
        .preview(
            trade_republic().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();
    service.commit(&preview, &ImportOptions::default()).unwrap();

    let transactions = store.transactions_for_account(&account.id).unwrap();
    let holdings = build_holdings(&transactions, "EUR", &store).unwrap();
    assert_eq!(holdings.income[0].gross_base, dec!(0.22));
    // 0.22 − 0.03 = 0.19, not 0.19 × 0.853898 = 0.162240.
    assert_eq!(holdings.dividends_base, dec!(0.19));
}

/// A saved layout is made from one export and applied to the next, which will hold wordings
/// the layout has never seen — Saxo prints the operation as "Sell 3 @ 139.74 USD", a string
/// no list can enumerate. The layout must keep reading the file it is applied to.
#[test]
fn a_saved_layout_still_reads_the_wordings_it_was_never_taught() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Saxo", "EUR");

    let csv = "Trade Date,Event,Amount,Instrument currency\n\
               02-Jan-2025,Sell 3 @ 139.74 USD,419.22,USD\n\
               02-Jan-2025,Custody Fee,-3.91,USD\n\
               02-Apr-2025,Dividend,1.83,EUR\n";
    let headers: Vec<String> = vec![
        "Trade Date".into(),
        "Event".into(),
        "Amount".into(),
        "Instrument currency".into(),
    ];

    // The layout as a template would carry it: columns and the account, no wordings at all.
    let mapping = ImportMapping::detect(&headers).with_account(&account.id);
    assert!(
        mapping.kind_of("Sell 3 @ 139.74 USD").is_none(),
        "nothing taught yet"
    );

    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    let kinds: Vec<TransactionKind> = preview
        .rows
        .iter()
        .map(|r| r.draft.as_ref().unwrap().kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TransactionKind::Sell,
            TransactionKind::Fee,
            TransactionKind::Dividend
        ]
    );
    // What was read is handed back in the mapping, so the wizard shows it and can override it.
    assert_eq!(
        preview.mapping.kind_of("Sell 3 @ 139.74 USD"),
        Some(TransactionKind::Sell)
    );
    assert!(preview.unknown_kinds().is_empty());
}
