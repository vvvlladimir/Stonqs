use super::*;

/// Card spending and refunds share one kind; amount sign distinguishes them.
/// The fixture is large enough for sign-consistency detection.
fn card_statement() -> String {
    let mut csv = String::from("date,type,amount,currency,description\n");
    csv.push_str("2026-01-05,TRANSFER_INBOUND,500.00,EUR,Incoming transfer\n");
    for day in 6..=16 {
        csv.push_str(&format!(
            "2026-01-{day:02},CARD_TRANSACTION,-10.00,EUR,Supermarket\n"
        ));
    }
    csv.push_str("2026-02-20,CARD_TRANSACTION,36.64,EUR,Refund EUROFLORIST\n");
    csv
}

/// A card refund is income, not another expense: 500 − 11 × 10 + 36.64 = 426.64.
#[test]
fn a_card_refund_is_an_inflow_not_another_charge() {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Bank", "EUR");
    store.save_account(&cash).unwrap();

    let csv = card_statement();
    let headers: Vec<String> = csv
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let mapping = ImportMapping::detect(&headers)
        .with_account(&cash.id)
        .with_kind_alias("CARD_TRANSACTION", TransactionKind::Withdrawal);

    let service = ImportService::new(&store);
    let preview = service
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    // File-level decision: 12 of 13 rows agree with the sign.
    assert_eq!(preview.amount_sign, AmountSign::Signed);

    // The refund is parsed as income and reported explicitly.
    let refund = preview.rows.last().unwrap();
    assert_eq!(refund.status, RowStatus::Ready);
    assert_eq!(refund.draft.as_ref().unwrap().kind, TransactionKind::Deposit);
    assert_eq!(refund.problems[0].code, ProblemCode::DirectionFromSign);
    assert_eq!(refund.problems[0].severity, Severity::Warning);
    assert_eq!(preview.summary.warnings, 1);
    assert_eq!(preview.summary.invalid, 0);

    // A warning does not block writing.
    let result = service.commit(&preview, &ImportOptions::default()).unwrap();
    assert_eq!(result.imported, 13);

    let balances = sq_core::calc::cash_balances(
        &store.transactions_for_account(&cash.id).unwrap(),
        std::slice::from_ref(&cash),
        "2026-12-31".parse().unwrap(),
    );
    assert_eq!(balances[&cash.id]["EUR"], dec!(426.64));
}

/// An export may print absolute amounts while operation kind carries direction.
#[test]
fn a_file_without_signs_keeps_the_kind_as_written() {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Bank", "EUR");
    store.save_account(&cash).unwrap();

    let mut csv = String::from("date,type,amount,currency\n");
    for day in 1..=6 {
        csv.push_str(&format!("2026-03-{day:02},DEPOSIT,100.00,EUR\n"));
        csv.push_str(&format!("2026-03-{day:02},WITHDRAWAL,40.00,EUR\n"));
    }
    let headers: Vec<String> = csv
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let mapping = ImportMapping::detect(&headers).with_account(&cash.id);

    let preview = ImportService::new(&store)
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.amount_sign, AmountSign::Unsigned);
    assert_eq!(preview.summary.warnings, 0);
    // 6 × 100 − 6 × 40 = 360: withdrawals remain withdrawals.
    let kinds: Vec<_> = preview
        .rows
        .iter()
        .map(|r| r.draft.as_ref().unwrap().kind)
        .collect();
    assert_eq!(kinds[0], TransactionKind::Deposit);
    assert_eq!(kinds[1], TransactionKind::Withdrawal);
}

/// If half the signs disagree with kind, the mapping is wrong, not noisy.
#[test]
fn a_half_disagreeing_file_is_reported_and_left_alone() {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Bank", "EUR");
    store.save_account(&cash).unwrap();

    let mut csv = String::from("date,type,amount,currency\n");
    for day in 1..=6 {
        csv.push_str(&format!("2026-04-{day:02},DEPOSIT,100.00,EUR\n"));
        csv.push_str(&format!("2026-04-{day:02},DEPOSIT,-100.00,EUR\n"));
    }
    let headers: Vec<String> = csv
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let mapping = ImportMapping::detect(&headers).with_account(&cash.id);

    let preview = ImportService::new(&store)
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.amount_sign, AmountSign::Unsigned);
    assert!(
        preview
            .problems
            .iter()
            .any(|p| p.code == ProblemCode::AmountSignAmbiguous),
        "{:?}",
        preview.problems
    );
}

/// An explicit mapping overrides auto-detection and its warning.
#[test]
fn the_user_can_overrule_the_detected_sign() {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Bank", "EUR");
    store.save_account(&cash).unwrap();

    let csv = card_statement();
    let headers: Vec<String> = csv
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let mapping = ImportMapping::detect(&headers)
        .with_account(&cash.id)
        .with_kind_alias("CARD_TRANSACTION", TransactionKind::Withdrawal)
        .with_amount_sign(AmountSign::Unsigned);

    let preview = ImportService::new(&store)
        .preview(csv.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();

    assert_eq!(preview.amount_sign, AmountSign::Unsigned);
    let refund = preview.rows.last().unwrap();
    assert_eq!(refund.draft.as_ref().unwrap().kind, TransactionKind::Withdrawal);
    assert_eq!(preview.summary.warnings, 0);
}

/// A crypto statement, where value mostly moves *inside* the portfolio: a balance
/// conversion is one wording on two rows, and only the sign tells the legs apart. Income
/// that is neither a dividend nor interest — cashback, staking rewards — has to land
/// somewhere it can be counted.
const CRYPTO_STATEMENT: &str = "\
Timestamp (UTC),Transaction Kind,Currency,Amount
2024-08-02 08:18:46,USDT (SOL) Deposit,USDT,78.218523
2024-08-03 08:18:46,USDT (SOL) Deposit,USDT,10
2024-08-04 08:18:46,USDT (SOL) Deposit,USDT,10
2024-08-05 08:18:46,USDT (SOL) Deposit,USDT,10
2024-08-06 08:18:46,crypto_withdrawal,USDT,-5
2025-03-31 12:53:56,Balance Conversion,USDC,81.161849
2025-03-31 12:53:56,Balance Conversion,USDT,-81.161849
2024-08-08 06:28:12,Card Cashback,CRO,58.906197
2024-01-15 04:02:52,CRO Lockup Rewards,CRO,5.15453552
";

#[test]
fn a_conversion_is_two_legs_of_one_wording_and_a_reward_is_income() {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Crypto.com", "EUR");
    let day = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    store
        .save_fx_rates(&[
            FxRate::new("USDT", "EUR", day, dec!(0.9)),
            FxRate::new("USDC", "EUR", day, dec!(0.9)),
            FxRate::new("CRO", "EUR", day, dec!(0.1)),
        ])
        .unwrap();

    // The wordings are recognised from the file's own values, so the mapping comes from the
    // first pass and only the account is added by hand.
    let service = ImportService::new(&store);
    let detected = service
        .preview(CRYPTO_STATEMENT.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();
    let mapping = detected.mapping.clone().with_account(&account.id);
    let preview = service
        .preview(
            CRYPTO_STATEMENT.as_bytes(),
            &ParseConfig::default(),
            Some(&mapping),
            &[],
        )
        .unwrap();

    // Deposits and the withdrawal agree with their signs, so the file is signed; the
    // conversion legs never voted, or the negative ones would have outvoted the rest.
    assert_eq!(preview.amount_sign, AmountSign::Signed);
    assert_eq!(preview.summary.invalid, 0, "{:?}", preview.rows);
    let kinds: Vec<TransactionKind> = preview
        .rows
        .iter()
        .map(|r| r.draft.as_ref().unwrap().kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TransactionKind::Deposit,
            TransactionKind::Deposit,
            TransactionKind::Deposit,
            TransactionKind::Deposit,
            TransactionKind::Withdrawal,
            TransactionKind::TransferIn,
            TransactionKind::TransferOut,
            TransactionKind::Cashback,
            TransactionKind::Reward,
        ]
    );

    service.commit(&preview, &ImportOptions::default()).unwrap();
    let transactions = store.transactions_for_accounts(&pair(&account), None).unwrap();
    let holdings = build_holdings(&transactions, "EUR", &store).unwrap();

    // USDT: 78.218523 + 10 + 10 + 10 - 5 - 81.161849 = 22.056674
    assert_eq!(holdings.cash["USDT"], dec!(22.056674));
    assert_eq!(holdings.cash["USDC"], dec!(81.161849));
    // CRO: 58.906197 + 5.15453552 = 64.06073252
    assert_eq!(holdings.cash["CRO"], dec!(64.06073252));

    // The two legs of one wording are linked at import, which is what tells them apart from
    // a bank transfer: a conversion moves nothing in or out of the portfolio, so it is not a
    // TWR flow — four deposits and one withdrawal are.
    let legs: Vec<&Transaction> = transactions
        .iter()
        .filter(|t| matches!(t.kind, TransactionKind::TransferIn | TransactionKind::TransferOut))
        .collect();
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].link_id, legs[1].link_id);
    assert!(legs[0].link_id.is_some());
    assert_eq!(holdings.external_flows.len(), 5);

    // Neither kind is a dividend or interest, and both are still counted.
    assert_eq!(holdings.dividends_base, dec!(0));
    assert_eq!(holdings.interest_base, dec!(0));
    let income = income_by_kind(&holdings.income);
    // 58.906197 CRO at 0.1 = 5.8906197
    assert_eq!(income[&TransactionKind::Cashback].net_base, dec!(5.8906197));
    // 5.15453552 CRO at 0.1 = 0.515453552
    assert_eq!(income[&TransactionKind::Reward].net_base, dec!(0.515453552));
}
