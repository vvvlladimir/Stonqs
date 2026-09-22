//! What a user's own export does that a broker's own never does: a position carried in from
//! another broker with no cost attached, a ticker that already names something else, a row
//! edited by hand after it was imported, a split applied part-way through the statement.
//!
//! Every one of these is a *warning* by design (`.claude/rules/import.md`): they are heuristics
//! over somebody else's file, and a false positive must never block an import.

use super::*;

/// Shares arriving with no money named. The lot's cost basis would be zero and the whole
/// position would read as profit, which is the single most expensive thing a broker change can
/// do to this app's numbers — so it is said, and the value can be typed into the row.
#[test]
fn shares_arriving_without_a_price_are_flagged_and_fixable() {
    const CSV: &str = "\
date,type,symbol,quantity,currency
2024-01-15,DELIVERY_INBOUND,AAPL,10,USD
";
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);

    let preview = service
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    let problems = &preview.rows[0].problems;
    assert!(
        problems
            .iter()
            .any(|p| p.code == ProblemCode::DeliveryWithoutCost && p.severity == Severity::Warning),
        "delivery with no value is a warning, not a refusal: {problems:?}"
    );

    // The file has no price column at all, so the repair is an added value rather than an edited
    // cell. An override of an unmapped field used to be dropped on the floor.
    assert_eq!(mapping.column(ImportField::Price), None);
    let fixed = service
        .preview(
            CSV.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[RowOverride::new(1, ImportField::Price, "185.50")],
        )
        .unwrap();
    assert_eq!(fixed.rows[0].draft.as_ref().unwrap().price, dec!(185.50));
    assert_eq!(fixed.rows[0].draft.as_ref().unwrap().amount, dec!(1855.00));
    assert!(
        !fixed.rows[0]
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::DeliveryWithoutCost)
    );
}

/// A ticker names a listing; an ISIN names the instrument. Two brokers printing "VUSA" for two
/// different funds must not pour one's trades into the other's position — and a ticker is unique
/// in the database, so the row waits for one of its own rather than being written either way.
#[test]
fn one_ticker_is_never_allowed_to_mean_two_instruments() {
    const CSV: &str = "\
date,type,symbol,isin,quantity,unit_price,currency
2024-01-15,BUY,VUSA,IE00BFMXXD54,10,80.00,EUR
";
    let (store, account) = store_with_account();
    store
        .save_security(&Security {
            isin: Some("IE00B3XXRP09".into()),
            ..Security::new("VUSA", "Vanguard S&P 500 (LSE)", "GBP", SecurityKind::Etf)
        })
        .unwrap();

    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = service
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.rows[0].status, RowStatus::Invalid);
    assert!(
        preview.rows[0]
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::TickerIsinConflict)
    );
    // Nothing is written and the stored instrument keeps its own ISIN.
    let result = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(
        store.find_security_by_symbol("VUSA").unwrap().unwrap().isin,
        Some("IE00B3XXRP09".into())
    );
}

/// The same ISIN under a ticker the database has never seen is the *ordinary* case — a foreign
/// export naming another venue's listing — and it must still join the instrument it names.
#[test]
fn the_isin_is_what_joins_a_row_to_an_instrument() {
    const CSV: &str = "\
date,type,symbol,isin,quantity,unit_price,currency
2024-01-15,BUY,VUAA.DE,IE00B3XXRP09,10,80.00,EUR
";
    let (store, account) = store_with_account();
    let stored = Security {
        isin: Some("IE00B3XXRP09".into()),
        ..Security::new("VUSA", "Vanguard S&P 500", "GBP", SecurityKind::Etf)
    };
    store.save_security(&stored).unwrap();

    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = service
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.rows[0].status, RowStatus::Ready);
    assert_eq!(
        preview.rows[0].draft.as_ref().unwrap().security_id.as_deref(),
        Some(stored.id.as_str())
    );
}

/// A row corrected by hand after it was imported no longer matches its own file by content, so
/// re-importing that file would write it a second time. It is recognised by what cannot have
/// been edited — day, account, instrument, quantity — and left out unless asked for.
#[test]
fn a_row_edited_after_import_is_not_written_a_second_time() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-01-15,BUY,AAPL,10,185.50,USD
";
    let (store, account) = store_with_account();
    let apple = Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock);
    store.save_security(&apple).unwrap();
    // What the first import wrote, with the price corrected afterwards by hand.
    store
        .save_transaction(&Transaction::buy(
            &account.id,
            &apple.id,
            "2024-01-15".parse().unwrap(),
            dec!(10),
            dec!(186.12),
            "USD",
        ))
        .unwrap();

    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = service
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.rows[0].status, RowStatus::Similar);
    assert_eq!(preview.summary.similar, 1);
    let result = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(result.similar, 1);

    // Two trades of the same size on one day at two prices do happen; saying so writes it.
    let result = service
        .commit(
            &preview,
            &ImportOptions {
                import_similar: true,
                ..ImportOptions::default()
            },
        )
        .unwrap();
    assert_eq!(result.imported, 1);
}

/// One instrument's prices stepping by a whole factor between two adjacent trades: the broker
/// applied a split mid-statement, so the quantities on either side mean different shares.
#[test]
fn prices_stepping_by_a_whole_factor_are_called_out() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-05-01,BUY,NVDA,2,900.00,USD
2024-07-01,BUY,NVDA,20,90.00,USD
";
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = service
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    let split: Vec<_> = preview
        .problems
        .iter()
        .filter(|p| p.code == ProblemCode::PossibleSplit)
        .collect();
    assert_eq!(
        split.len(),
        1,
        "one notice per instrument: {:?}",
        preview.problems
    );
    assert_eq!(split[0].severity, Severity::Warning);
    assert_eq!(split[0].params.get("ratio").map(String::as_str), Some("10"));
}

/// A market move is not a split. Doubling over two months is ordinary, and a notice on it would
/// train the user to ignore the one that matters.
#[test]
fn an_ordinary_price_move_is_not_called_a_split() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-05-01,BUY,AAPL,2,185.50,USD
2024-07-01,BUY,AAPL,2,214.30,USD
";
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = ImportService::new(&store)
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert!(
        !preview
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::PossibleSplit)
    );
}

/// Money landing on an account that keeps another currency. Legal — a multi-currency account is
/// a real thing — and also exactly what a mis-read currency column looks like, so it is said
/// once per row and never blocks. A trade is not judged this way: the depot keeps no money.
#[test]
fn a_payment_in_another_currency_than_the_account_is_pointed_out() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency,amount
2024-03-01,DEPOSIT,,,,CHF,5000
2024-03-02,BUY,AAPL,10,185.50,USD,1855
";
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = ImportService::new(&store)
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    let code = |row: usize| {
        preview.rows[row]
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::AccountCurrencyMismatch)
    };
    assert!(
        code(0),
        "CHF onto a USD cash account: {:?}",
        preview.rows[0].problems
    );
    assert!(!code(1), "the purchase is in the account's own currency");
    assert_ne!(preview.rows[0].status, RowStatus::Invalid);
}
