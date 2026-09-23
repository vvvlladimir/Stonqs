use super::*;
use sq_core::import::{Emit, ImportRule, Sign, Test};

/// One broker line can be more than one operation, or none at all.
const FILE: &str = "\
date,type,symbol,quantity,unit_price,currency,amount,transaction id
2024-03-01,REINVEST DIVIDEND,AAPL,0.5,80.00,USD,40.00,TR-1
2024-03-05,MONTHLY STATEMENT,,,,USD,0,TR-2
2024-03-10,WALLET MOVE,,,,USD,100.00,TR-3
2024-03-15,ADJUSTMENT,,,,USD,-15.00,TR-4
2024-03-16,ADJUSTMENT,,,,USD,25.00,TR-5
";

fn mapping(account: &Account) -> ImportMapping {
    ImportMapping::detect(&headers_of(FILE))
        .with_account(&account.id)
        // A reinvested dividend is an income and a purchase: the money arrives and is spent.
        .with_rule(
            ImportRule::when(ImportField::Kind, Test::Contains("reinvest".into()))
                .emit(Emit::of(TransactionKind::Dividend).with(ImportField::Quantity, "0"))
                .emit(Emit::of(TransactionKind::Buy)),
        )
        // A line that is not an operation at all.
        .with_rule(ImportRule::when(
            ImportField::Kind,
            Test::Contains("statement".into()),
        ))
        // Money moving between the user's own wallets: two legs of one move.
        .with_rule(
            ImportRule::when(ImportField::Kind, Test::Equals("wallet move".into()))
                .emit(Emit::of(TransactionKind::TransferOut))
                .emit(Emit::of(TransactionKind::TransferIn))
                .linked(),
        )
        // One wording, two directions, told apart by the sign of the amount.
        .with_rule(
            ImportRule::when(ImportField::Kind, Test::Equals("adjustment".into()))
                .and(ImportField::Amount, Test::Sign(Sign::Negative))
                .emit(Emit::of(TransactionKind::Withdrawal)),
        )
        .with_rule(
            ImportRule::when(ImportField::Kind, Test::Equals("adjustment".into()))
                .emit(Emit::of(TransactionKind::Deposit)),
        )
}

fn preview_of(store: &Store, account: &Account) -> sq_core::import::ImportPreview {
    ImportService::new(store)
        .preview(
            FILE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping(account)),
            &[],
        )
        .unwrap()
}

#[test]
fn one_row_becomes_the_operations_its_rule_names() {
    let (store, account) = store_with_account();
    store
        .save_security(&Security::new("AAPL", "Apple", "USD", SecurityKind::Stock))
        .unwrap();
    let preview = preview_of(&store, &account);

    // Five file rows, seven operations: two of them produced a pair, one produced none.
    assert_eq!(preview.rows.len(), 7);

    let reinvest: Vec<_> = preview.rows.iter().filter(|r| r.number == 1).collect();
    assert_eq!(reinvest.len(), 2);
    assert_eq!(reinvest[0].part, 1);
    assert_eq!(reinvest[1].part, 2);

    let income = reinvest[0].draft.as_ref().unwrap();
    let purchase = reinvest[1].draft.as_ref().unwrap();
    assert_eq!(income.kind, TransactionKind::Dividend);
    assert_eq!(income.amount, dec!(40.00));
    assert_eq!(income.quantity, dec!(0), "the income leg buys nothing");
    assert_eq!(purchase.kind, TransactionKind::Buy);
    assert_eq!(purchase.quantity, dec!(0.5));
    assert_eq!(purchase.price, dec!(80.00));

    // The broker named the row once, so each half is named apart — a later restatement of
    // either half still finds the operation it belongs to.
    assert_eq!(income.external_id.as_deref(), Some("TR-1#1"));
    assert_eq!(purchase.external_id.as_deref(), Some("TR-1#2"));
}

#[test]
fn a_rule_that_produces_nothing_leaves_the_row_out() {
    let (store, account) = store_with_account();
    let preview = preview_of(&store, &account);

    let statement = preview.rows.iter().find(|r| r.number == 2).unwrap();
    assert_eq!(statement.status, RowStatus::Ignored);
    assert!(statement.draft.is_none());
    assert_eq!(preview.summary.ignored, 1);
}

#[test]
fn the_legs_of_one_internal_move_carry_the_same_link() {
    let (store, account) = store_with_account();
    let preview = preview_of(&store, &account);

    let legs: Vec<_> = preview
        .rows
        .iter()
        .filter(|r| r.number == 3)
        .filter_map(|r| r.draft.as_ref())
        .collect();
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].kind, TransactionKind::TransferOut);
    assert_eq!(legs[1].kind, TransactionKind::TransferIn);
    assert_eq!(legs[0].link_id, legs[1].link_id);
    assert!(legs[0].link_id.is_some(), "a pair that is internal says so");
}

#[test]
fn a_condition_on_the_sign_decides_the_direction_instead_of_the_wording() {
    let (store, account) = store_with_account();
    let preview = preview_of(&store, &account);

    let out = preview.rows.iter().find(|r| r.number == 4).unwrap();
    let inbound = preview.rows.iter().find(|r| r.number == 5).unwrap();
    assert_eq!(out.draft.as_ref().unwrap().kind, TransactionKind::Withdrawal);
    assert_eq!(inbound.draft.as_ref().unwrap().kind, TransactionKind::Deposit);
    // The rule answered the direction, so nothing flipped it a second time.
    assert!(
        out.problems
            .iter()
            .all(|p| p.code != ProblemCode::DirectionFromSign)
    );
}
