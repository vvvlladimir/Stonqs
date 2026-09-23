use super::*;

/// A broker that prints what the account moved, not what the trade was worth: the amount
/// already has the commission in it, so the stored gross has to be restored from it.
/// Buys:  10 × 100.00 = 1000.00 paid as 1004.95 (4.95 commission)
///        5 × 210.00 = 1050.00 paid as 1054.95
///        2 × 300.00 =  600.00 paid as  604.95
/// Sell:   4 × 120.00 =  480.00 received as 476.05 (3.95 commission)
fn net_style() -> &'static str {
    "\
date,type,symbol,quantity,unit_price,currency,fee,amount
2024-01-15,BUY,AAPL,10,100.00,USD,4.95,1004.95
2024-02-15,BUY,AAPL,5,210.00,USD,4.95,1054.95
2024-03-15,BUY,AAPL,2,300.00,USD,4.95,604.95
2024-04-15,SELL,AAPL,4,120.00,USD,3.95,476.05
"
}

#[test]
fn an_amount_that_already_carries_the_commission_is_read_back_as_gross() {
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(net_style())).with_account(&account.id);

    let service = ImportService::new(&store);
    let preview = service
        .preview(
            net_style().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    assert_eq!(preview.amount_basis, AmountBasis::Net);

    let drafts: Vec<_> = preview.rows.iter().filter_map(|r| r.draft.as_ref()).collect();
    assert_eq!(drafts[0].amount, dec!(1000.00), "1004.95 − 4.95 commission");
    assert_eq!(drafts[0].fees, dec!(4.95));
    assert_eq!(drafts[3].amount, dec!(480.00), "476.05 + 3.95 commission");

    // What the account moved is the trade plus its commission again: 1000.00 + 4.95.
    assert_eq!(drafts[0].amount + drafts[0].fees, dec!(1004.95));
}

/// The same file written the other way round: the amount is the trade's own value and the
/// commission sits beside it. Nothing is restored, and the totals are identical.
#[test]
fn a_gross_amount_is_left_exactly_as_the_file_wrote_it() {
    let gross = "\
date,type,symbol,quantity,unit_price,currency,fee,amount
2024-01-15,BUY,AAPL,10,100.00,USD,4.95,1000.00
2024-02-15,BUY,AAPL,5,210.00,USD,4.95,1050.00
2024-03-15,BUY,AAPL,2,300.00,USD,4.95,600.00
2024-04-15,SELL,AAPL,4,120.00,USD,3.95,480.00
";
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(gross)).with_account(&account.id);

    let preview = ImportService::new(&store)
        .preview(gross.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.amount_basis, AmountBasis::Gross);
    let drafts: Vec<_> = preview.rows.iter().filter_map(|r| r.draft.as_ref()).collect();
    assert_eq!(drafts[0].amount, dec!(1000.00));
    assert_eq!(drafts[3].amount, dec!(480.00));
}

/// The reading is the user's to overrule: a file read as net imports as written once the
/// layout says gross.
#[test]
fn the_file_reading_can_be_overruled_by_the_layout() {
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(net_style()))
        .with_account(&account.id)
        .with_amount_basis(AmountBasis::Gross);

    let preview = ImportService::new(&store)
        .preview(
            net_style().as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    assert_eq!(preview.amount_basis, AmountBasis::Gross);
    let draft = preview.rows[0].draft.as_ref().unwrap();
    assert_eq!(draft.amount, dec!(1004.95));
}
