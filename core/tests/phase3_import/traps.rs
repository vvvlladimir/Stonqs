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

/// One instrument's prices stepping by a whole factor between two trades weeks apart: the
/// broker applied a split mid-statement, so the quantities on either side mean different shares.
#[test]
fn prices_stepping_by_a_whole_factor_are_called_out() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-05-01,BUY,NVDA,2,900.00,USD
2024-06-20,BUY,NVDA,20,90.00,USD
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

/// A market move is not a split, and the two are told apart by exactness and by time. A real
/// case: a semiconductor ETF bought twice a year apart, 9.386 then 19.06 — a factor of 2.03,
/// which is the index doubling, not a corporate action. Calling that a split once teaches the
/// user to ignore the notice when it is real.
#[test]
fn an_ordinary_price_move_is_not_called_a_split() {
    const NEARLY_DOUBLED: &str = "\
date,type,symbol,quantity,unit_price,currency
2025-06-02,BUY,SEMI.AS,10,9.3860000000,EUR
2026-06-02,BUY,SEMI.AS,10,19.0600000000,EUR
";
    const ORDINARY: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-05-01,BUY,AAPL,2,185.50,USD
2024-07-01,BUY,AAPL,2,214.30,USD
";
    let (store, account) = store_with_account();
    let flagged = |csv: &str| {
        let mapping = ImportMapping::detect(&headers_of(csv)).with_account(&account.id);
        ImportService::new(&store)
            .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
            .unwrap()
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::PossibleSplit)
    };

    assert!(
        !flagged(NEARLY_DOUBLED),
        "2.03 over a year is the market, not a split"
    );
    assert!(!flagged(ORDINARY));

    // The same two prices weeks apart, and exact, still are a split.
    assert!(flagged(
        "\
date,type,symbol,quantity,unit_price,currency
2026-05-02,BUY,SEMI.AS,10,19.0600000000,EUR
2026-06-02,BUY,SEMI.AS,20,9.5300000000,EUR
"
    ));
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

/// A sale is checked against the stored ledger and the file together: 5 AAPL stored, then the
/// file sells 3 (5 − 3 = 2 left) and 4 more — 2 held, 4 sold, 2 missing. The first sale is
/// covered and says nothing; the second is a warning naming what is missing.
#[test]
fn a_sale_of_more_than_the_ledger_holds_is_called_out() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency
2024-03-01,SELL,AAPL,3,190.00,USD
2024-04-01,SELL,AAPL,4,195.00,USD
";
    let (store, account) = store_with_account();
    let aapl = Security::new("AAPL", "Apple", "USD", SecurityKind::Stock);
    store.save_security(&aapl).unwrap();
    store
        .save_transaction(&Transaction::buy(
            &account.id,
            &aapl.id,
            sq_core_date(2024, 1, 10),
            dec!(5),
            dec!(180),
            "USD",
        ))
        .unwrap();

    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = ImportService::new(&store)
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    let gaps: Vec<_> = preview
        .rows
        .iter()
        .flat_map(|r| &r.problems)
        .filter(|p| p.code == ProblemCode::SaleExceedsHoldings)
        .collect();
    assert_eq!(gaps.len(), 1, "{:?}", preview.rows);
    assert_eq!(gaps[0].row, Some(2));
    assert_eq!(gaps[0].severity, Severity::Warning);
    assert_eq!(gaps[0].params["held"], "2");
    assert_eq!(gaps[0].params["missing"], "2");
    // A warning, not a refusal: the row is still written.
    assert_eq!(preview.rows[1].status, RowStatus::Ready);
}

/// The likeliest place for the missing receipt is a row of the same file that will not be
/// written — a wording nobody mapped — so the warning names it.
#[test]
fn an_unread_row_of_the_same_instrument_is_named_as_the_likely_cause() {
    const CSV: &str = "\
date,type,symbol,quantity,unit_price,currency,amount
2026-04-19,FREE_RECEIPT,XLM,580,0.15,EUR,
2026-04-19,SELL,XLM,580,0.1458524,EUR,84.59
";
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let preview = ImportService::new(&store)
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    let gap = preview.rows[1]
        .problems
        .iter()
        .find(|p| p.code == ProblemCode::SaleExceedsHoldings)
        .expect("the sale is uncovered while the receipt is unread");
    assert_eq!(gap.params["unread"], "1");
    assert_eq!(gap.params["unread_kinds"], "FREE_RECEIPT");

    // Mapped, the receipt covers the sale on the same day and the warning is gone.
    let mapping = mapping.with_kind_alias("FREE_RECEIPT", TransactionKind::DeliveryInbound);
    let preview = ImportService::new(&store)
        .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert!(
        preview
            .rows
            .iter()
            .flat_map(|r| &r.problems)
            .all(|p| p.code != ProblemCode::SaleExceedsHoldings),
        "{:?}",
        preview.rows
    );
}

fn sq_core_date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// `Decimal` holds 7.9e28 and its operators panic instead of saturating, so a cell near that
/// ceiling used to take down the preview on the first `quantity × price`. The figure is refused
/// where it is read, and every later multiplication is then over numbers that fit.
#[test]
fn a_number_too_large_to_calculate_with_refuses_its_row_instead_of_panicking() {
    const CSV: &str = "\
date,type,symbol,isin,quantity,unit_price,amount,currency,fee
2024-01-02,BUY,AAPL,US0378331005,79228162514264337593543950335,79228162514264337593543950335,1,USD,0
";
    let (store, account) = store_with_account();
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
            .any(|p| p.code == ProblemCode::NumberOutOfRange && p.severity == Severity::Error),
        "an unusable figure is an error on its row: {:?}",
        preview.rows[0].problems
    );

    // Nothing reaches the ledger, so no later screen can meet it either.
    let options = ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    };
    let result = service.commit(&preview, &options).unwrap();
    assert_eq!(result.imported, 0);
}

/// A ticker is renamed (FB became META) and the broker's export prints the old one on the old
/// rows and the new one on the new ones. The ISIN says it is one instrument throughout, and the
/// preview reads it that way — so the commit has to as well, or it creates two instruments, the
/// next preview joins the old rows to the *other* one, and re-importing the same file writes
/// them a second time.
#[test]
fn one_isin_under_two_tickers_is_one_instrument_and_re_importing_is_a_no_op() {
    const CSV: &str = "\
date,type,symbol,isin,quantity,unit_price,amount,currency
2022-01-03,BUY,FB,US30303M1027,10,300,3000,USD
2023-01-03,BUY,META,US30303M1027,5,120,600,USD
2023-06-01,DIVIDEND,META,US30303M1027,,,10,USD
";
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let options = ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    };
    let import = || {
        let preview = service
            .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
            .unwrap();
        service.commit(&preview, &options).unwrap()
    };

    assert_eq!(import().imported, 3);
    let securities = store.list_securities().unwrap();
    assert_eq!(securities.len(), 1, "one ISIN is one instrument: {securities:?}");

    assert_eq!(import().imported, 0, "the same file twice writes nothing twice");
    assert_eq!(store.list_securities().unwrap().len(), 1);
}

/// One row of the same export is missing the ticker its neighbours carry — a cell the broker
/// left empty. Its ISIN names the instrument the other rows created, so it is written against
/// that instrument; written without one, it reached the ledger as something the next preview
/// could not find, and the same file imported twice.
#[test]
fn a_row_with_an_isin_and_no_ticker_joins_the_instrument_its_neighbours_made() {
    const CSV: &str = "\
date,type,symbol,isin,quantity,unit_price,amount,currency
2024-01-03,BUY,XDWT,IE00BM67HT60,2,84.50,169.00,EUR
2024-02-03,BUY,,IE00BM67HT60,1,86.00,86.00,EUR
";
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mapping = ImportMapping::detect(&headers_of(CSV)).with_account(&account.id);
    let options = ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    };
    let import = || {
        let preview = service
            .preview(CSV.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
            .unwrap();
        service.commit(&preview, &options).unwrap()
    };

    assert_eq!(import().imported, 2);
    let securities = store.list_securities().unwrap();
    assert_eq!(securities.len(), 1);
    let held = build_holdings(
        &store.transactions_for_accounts(&pair(&account), None).unwrap(),
        "EUR",
        &store,
    )
    .unwrap();
    assert_eq!(held.positions[&securities[0].id].quantity, dec!(3));

    assert_eq!(import().imported, 0, "the same file twice writes nothing twice");
}
