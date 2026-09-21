use super::*;

/// Trade Republic changed its export mid-history: the older rows carry plain dates and
/// comma decimals, the newer ones ISO timestamps and dot decimals — in one file.
/// Amounts as written: -10,84  0,01  5,00  -24.999996  86.714375. A draft carries the
/// magnitude and leaves direction to the kind, so they sum to
/// 10.84 + 0.01 + 5.00 + 24.999996 + 86.714375 = 127.564371
const BROKER_CHANGED_ITS_SHAPE: &str = "\
date;type;amount
2024-11-30;WITHDRAWAL;-10,84
2024-11-26;DEPOSIT;0,01
2024-07-23;DEPOSIT;5,00
2025-01-16T16:13:36;WITHDRAWAL;-24.999996
2024-12-10T10:25:01;DEPOSIT;86.714375
";

/// Neither the odd dates nor the odd decimals may cost a row: they are warnings, and the
/// values stay exact. Trusting the file-level separator would read -24999996 here.
#[test]
fn a_file_that_changes_shape_halfway_imports_every_row() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Trade Republic", "EUR");

    let headers: Vec<String> = vec!["date".into(), "type".into(), "amount".into()];
    let mapping = ImportMapping::detect(&headers)
        .with_account(&account.id)
        .with_default_currency("EUR");
    let service = ImportService::new(&store);
    let preview = service
        .preview(
            BROKER_CHANGED_ITS_SHAPE.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    // The majority shape is detected; the two rows that disagree are parsed anyway.
    assert_eq!(preview.config.date_format.as_deref(), Some("%Y-%m-%d"));
    assert_eq!(preview.config.decimal_separator, Some(','));
    assert_eq!(preview.summary.invalid, 0, "{:?}", preview.rows);
    assert_eq!(preview.summary.ready, 5);

    let total: rust_decimal::Decimal = preview
        .rows
        .iter()
        .map(|r| r.draft.as_ref().unwrap().amount)
        .sum();
    assert_eq!(total, dec!(127.564371));
    // The dot-decimal row keeps every digit: the comma of the file must not be applied.
    assert_eq!(preview.rows[3].draft.as_ref().unwrap().amount, dec!(24.999996));

    let fallbacks = preview
        .rows
        .iter()
        .flat_map(|r| &r.problems)
        .filter(|p| p.code == ProblemCode::BadDate)
        .count();
    assert_eq!(fallbacks, 2);
    assert!(
        preview
            .rows
            .iter()
            .flat_map(|r| &r.problems)
            .all(|p| p.severity == Severity::Warning)
    );
}

/// Most broker files name no currency: the account the row lands on carries it, so asking
/// the user to type it again would be asking twice.
#[test]
fn a_file_without_a_currency_column_takes_the_accounts_currency() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Swissquote", "CHF");

    let csv = "Date;Transaction;Quantity;Net Amount\n\
               24-08-2022;Deposit;;2000.00\n";
    let headers: Vec<String> = vec![
        "Date".into(),
        "Transaction".into(),
        "Quantity".into(),
        "Net Amount".into(),
    ];
    let mapping = ImportMapping::detect(&headers).with_account(&account.id);
    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(mapping.column(ImportField::Currency), None);
    assert_eq!(preview.summary.ready, 1, "{:?}", preview.rows);
    let draft = preview.rows[0].draft.as_ref().unwrap();
    assert_eq!(draft.currency, "CHF");
    assert_eq!(draft.amount, dec!(2000.00));
}

/// A broker prints lines that are not operations: Schwab's "Name Change" and
/// "Journaled Shares" have no counterpart in the model. Refusing the whole file over them
/// is not an answer, and neither is importing them as something they are not.
#[test]
fn a_kind_the_user_chooses_to_skip_leaves_the_rest_importable() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Schwab", "USD");

    let csv = "Date,Action,Amount\n\
               11/01/2023,Deposit,1000\n\
               11/02/2023,Name Change,\n\
               11/03/2023,Name Change,\n";
    let headers: Vec<String> = vec!["Date".into(), "Action".into(), "Amount".into()];
    let detected = ImportMapping::detect(&headers).with_account(&account.id);
    let service = ImportService::new(&store);

    // Unmapped, the two lines are errors and the file cannot be imported as it stands.
    let before = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&detected), &[])
        .unwrap();
    assert_eq!(before.summary.invalid, 2);
    assert_eq!(before.unknown_kinds(), vec!["Name Change"]);

    let mapping = detected.with_ignored_kind("Name Change");
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(preview.summary.ready, 1);
    assert_eq!(preview.summary.ignored, 2);
    assert_eq!(preview.summary.invalid, 0, "{:?}", preview.rows);
    assert!(
        preview.unknown_kinds().is_empty(),
        "the value is decided, not unknown"
    );
    // The choice stays visible next to the other operation values, so it can be taken back.
    let skipped = preview.kinds.iter().find(|k| k.value == "Name Change").unwrap();
    assert!(skipped.ignored);
    assert_eq!(skipped.count, 2);

    let result = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 1);
    assert_eq!(result.skipped, 2);
    assert_eq!(
        store
            .transactions_for_account(account.reference_account_id.as_ref().unwrap())
            .unwrap()
            .len(),
        1
    );
}

/// Schwab prints a split as two rows of one wording, signed on the quantity: no money moves,
/// so the amount has no sign to give.
#[test]
fn a_split_printed_as_two_legs_lands_as_shares_in_and_shares_out() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Schwab", "USD");
    let security = Security::new("TNXP", "Tonix Pharmaceuticals", "USD", SecurityKind::Stock);
    store.save_security(&security).unwrap();

    // Five deposits and a withdrawal make the file signed; the split legs never vote.
    let csv = "Date,Action,Symbol,Quantity,Amount\n\
               06/01/2024,Deposit,,,1000\n\
               06/02/2024,Deposit,,,1000\n\
               06/03/2024,Deposit,,,1000\n\
               06/04/2024,Deposit,,,1000\n\
               06/05/2024,Deposit,,,1000\n\
               06/06/2024,Wire Sent,,,-500\n\
               06/10/2024,Reverse Split,TNXP,1,\n\
               06/10/2024,Reverse Split,TNXP,-10,\n";
    let headers: Vec<String> = vec![
        "Date".into(),
        "Action".into(),
        "Symbol".into(),
        "Quantity".into(),
        "Amount".into(),
    ];
    let mapping = ImportMapping::detect(&headers)
        .with_account(&account.id)
        .with_kind_alias("Reverse Split", TransactionKind::DeliveryInbound)
        .with_kind_alias("Wire Sent", TransactionKind::Withdrawal);

    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(preview.amount_sign, AmountSign::Signed);

    let split_in = preview.rows[6].draft.as_ref().unwrap();
    let split_out = preview.rows[7].draft.as_ref().unwrap();
    assert_eq!(split_in.kind, TransactionKind::DeliveryInbound);
    assert_eq!(split_in.quantity, dec!(1));
    assert_eq!(split_out.kind, TransactionKind::DeliveryOutbound);
    // The model never carries a negative quantity: the sign became the direction.
    assert_eq!(split_out.quantity, dec!(10));
}
