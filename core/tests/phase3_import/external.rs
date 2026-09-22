use super::*;

/// The broker names every row, so a second export of the same month writes nothing new.
const FIRST: &str = "\
date,type,symbol,quantity,unit_price,currency,fee,amount,transaction id
2024-01-15,BUY,AAPL,10,100.00,USD,4.95,1000.00,TR-8801
2024-02-15,BUY,AAPL,5,210.00,USD,4.95,1050.00,TR-8802
";

/// The same statement after the broker corrected the second trade's price.
const RESTATED: &str = "\
date,type,symbol,quantity,unit_price,currency,fee,amount,transaction id
2024-01-15,BUY,AAPL,10,100.00,USD,4.95,1000.00,TR-8801
2024-02-15,BUY,AAPL,5,212.00,USD,4.95,1060.00,TR-8802
";

fn imported<'a>(store: &'a Store, account: &Account, csv: &str) -> (ImportService<'a>, ImportMapping) {
    let mapping = ImportMapping::detect(&headers_of(csv)).with_account(&account.id);
    (ImportService::new(store), mapping)
}

#[test]
fn a_restated_row_replaces_the_one_it_names_instead_of_joining_it() {
    let (store, account) = store_with_account();
    store
        .save_security(&sq_core::model::Security::new(
            "AAPL",
            "Apple",
            "USD",
            SecurityKind::Stock,
        ))
        .unwrap();

    let (service, mapping) = imported(&store, &account, FIRST);
    let preview = service
        .preview(FIRST.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(
        preview.rows[0].draft.as_ref().unwrap().external_id.as_deref(),
        Some("TR-8801")
    );
    let first = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(first.imported, 2);

    // The same file again: both rows are already there, by name.
    let (service, mapping) = imported(&store, &account, FIRST);
    let again = service
        .preview(FIRST.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(again.summary.duplicates, 2);
    assert_eq!(again.summary.updated, 0);

    // The corrected export: one row is unchanged, the other restates what is stored.
    let (service, mapping) = imported(&store, &account, RESTATED);
    let corrected = service
        .preview(RESTATED.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(corrected.summary.duplicates, 1);
    assert_eq!(corrected.summary.updated, 1);
    assert_eq!(corrected.rows[1].status, RowStatus::Updated);
    assert_eq!(corrected.rows[1].problems[0].code, ProblemCode::RestatedInStore);
    assert_eq!(corrected.rows[1].problems[0].severity, Severity::Warning);

    let result = service.commit(&corrected, &ImportOptions::default()).unwrap();
    assert_eq!(result.updated, 1);
    assert_eq!(result.imported, 0);

    // Still two operations, and the second one now says 1060.00.
    let rows = store.transactions_for_account(&account.id).unwrap();
    assert_eq!(rows.len(), 2);
    let restated = rows
        .iter()
        .find(|t| t.external_id.as_deref() == Some("TR-8802"))
        .unwrap();
    assert_eq!(restated.amount, dec!(1060.00));
    assert_eq!(restated.price, dec!(212.00));
}
