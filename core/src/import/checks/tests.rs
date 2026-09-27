use super::*;

fn vote_of(rows: &[(TransactionKind, Decimal)]) -> SignVote {
    let mut vote = SignVote::default();
    for (kind, amount) in rows {
        count_vote(&mut vote, Some(*kind), *amount);
    }
    vote
}

#[test]
fn a_share_movement_is_signed_on_the_quantity_not_on_the_amount() {
    // A split is printed as two legs of one wording: "-1 share out, +8 shares in". No
    // money moves, so the amount cannot carry the direction — the quantity does.
    assert_eq!(
        resolve_direction(TransactionKind::DeliveryInbound, dec!(-1), AmountSign::Signed),
        Direction::Flipped(TransactionKind::DeliveryOutbound)
    );
    assert_eq!(
        resolve_direction(TransactionKind::DeliveryInbound, dec!(8), AmountSign::Signed),
        Direction::Keep
    );
    // A sale is signed too, but its direction is already in the kind: Buy and Sell never
    // turn into each other.
    assert_eq!(
        resolve_direction(TransactionKind::Sell, dec!(-100), AmountSign::Signed),
        Direction::Conflict
    );
    // Nothing is read into a file that does not sign its amounts.
    assert_eq!(
        resolve_direction(TransactionKind::DeliveryInbound, dec!(-1), AmountSign::Unsigned),
        Direction::Keep
    );
}

#[test]
fn conversions_do_not_vote_against_the_file_they_belong_to() {
    // A crypto file is half currency conversions, and one leg of every pair is negative
    // by construction. Counting those legs would drag the agreement under the threshold
    // and leave both legs credited — money out of nothing.
    let mut rows = vec![(TransactionKind::Deposit, dec!(100)); 5];
    rows.push((TransactionKind::Withdrawal, dec!(-40)));
    for _ in 0..9 {
        rows.push((TransactionKind::TransferIn, dec!(81.16)));
        rows.push((TransactionKind::TransferIn, dec!(-81.16)));
    }

    let vote = vote_of(&rows);
    assert_eq!(vote.votes, 6, "only the six rows whose direction is their own");
    let (sign, problem) = decide_amount_sign(vote);
    assert_eq!(sign, AmountSign::Signed);
    assert!(problem.is_none());
    // Not voting is not the same as not flipping: the negative leg still turns around.
    assert_eq!(
        resolve_direction(TransactionKind::TransferIn, dec!(-81.16), sign),
        Direction::Flipped(TransactionKind::TransferOut)
    );
}

#[test]
fn one_refund_among_many_charges_makes_the_file_signed() {
    let mut rows = vec![(TransactionKind::Withdrawal, dec!(-10)); 199];
    rows.push((TransactionKind::Withdrawal, dec!(36.64)));
    let (sign, problem) = decide_amount_sign(vote_of(&rows));
    assert_eq!(sign, AmountSign::Signed);
    assert!(problem.is_none());
    assert_eq!(
        resolve_direction(TransactionKind::Withdrawal, dec!(36.64), sign),
        Direction::Flipped(TransactionKind::Deposit)
    );
}

#[test]
fn all_positive_amounts_mean_the_sign_carries_nothing() {
    let rows = vec![
        (TransactionKind::Deposit, dec!(100)),
        (TransactionKind::Withdrawal, dec!(50)),
        (TransactionKind::Deposit, dec!(100)),
        (TransactionKind::Withdrawal, dec!(50)),
        (TransactionKind::Deposit, dec!(100)),
        (TransactionKind::Withdrawal, dec!(50)),
    ];
    let (sign, _) = decide_amount_sign(vote_of(&rows));
    assert_eq!(sign, AmountSign::Unsigned);
    assert_eq!(
        resolve_direction(TransactionKind::Withdrawal, dec!(50), sign),
        Direction::Keep
    );
}

#[test]
fn half_disagreeing_is_reported_and_changes_nothing() {
    let mut rows = vec![(TransactionKind::Deposit, dec!(-100)); 5];
    rows.extend(vec![(TransactionKind::Deposit, dec!(100)); 5]);
    let (sign, problem) = decide_amount_sign(vote_of(&rows));
    assert_eq!(sign, AmountSign::Unsigned);
    assert_eq!(problem.unwrap().code, ProblemCode::AmountSignAmbiguous);
}

#[test]
fn a_short_file_is_never_declared_signed() {
    let rows = vec![
        (TransactionKind::Withdrawal, dec!(-10)),
        (TransactionKind::Deposit, dec!(10)),
    ];
    assert_eq!(decide_amount_sign(vote_of(&rows)).0, AmountSign::Unsigned);
}

#[test]
fn a_trade_is_reported_never_flipped() {
    assert_eq!(
        resolve_direction(TransactionKind::Buy, dec!(200), AmountSign::Signed),
        Direction::Conflict
    );
}

#[test]
fn amount_is_checked_against_quantity_times_price() {
    let mut draft = draft_of(TransactionKind::Buy, dec!(199.99));
    draft.quantity = dec!(0.361938);
    draft.price = dec!(552.56);
    assert!(check_row(1, &draft, &CheckContext::default()).is_empty());

    draft.amount = dec!(19.99);
    let problems = check_row(1, &draft, &CheckContext::default());
    assert_eq!(problems[0].code, ProblemCode::AmountVsQuantityPrice);
    assert!(!problems[0].is_error());
}

#[test]
fn fx_rate_on_the_base_currency_is_reported() {
    let mut draft = draft_of(TransactionKind::Dividend, dec!(6.05));
    draft.fx_rate_to_base = Some(dec!(0.8539));
    let context = CheckContext {
        base_currency: Some("EUR"),
        today: None,
        account_currency: None,
    };
    let codes: Vec<_> = check_row(1, &draft, &context).iter().map(|p| p.code).collect();
    assert!(codes.contains(&ProblemCode::FxRateOnBaseCurrency));
}

fn draft_of(kind: TransactionKind, amount: Decimal) -> TransactionDraft {
    TransactionDraft {
        account_id: "acc".into(),
        kind,
        date: "2024-01-02".parse().unwrap(),
        symbol: None,
        isin: None,
        security_name: None,
        security_id: None,
        quantity: Decimal::ZERO,
        price: Decimal::ZERO,
        amount,
        fees: Decimal::ZERO,
        taxes: Decimal::ZERO,
        currency: "EUR".into(),
        fee_currency: None,
        tax_currency: None,
        fx_rate_to_base: None,
        link_id: None,
        external_id: None,
        replaces: None,
        note: None,
    }
}
