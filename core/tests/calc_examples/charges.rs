use super::*;

/// Standalone charges only: 2023 fees = 7 EUR; 2024 fee = 18 EUR, tax = 30 EUR,
/// total = 48 EUR (trade fee is already in cost basis).
#[test]
fn charges_by_year_counts_standalone_fees_only() {
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 3, 1), dec!(0.90));
    let mut buy = Transaction::buy(ACC, AAPL, d(2024, 4, 1), dec!(2), dec!(150), "EUR");
    buy.fees = dec!(9);

    let txns = vec![
        Transaction::cash(ACC, TransactionKind::Fee, d(2023, 6, 1), dec!(12), "EUR"),
        Transaction::cash(ACC, TransactionKind::FeeRefund, d(2023, 9, 1), dec!(5), "EUR"),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 3, 1), dec!(20), "USD"),
        Transaction::cash(ACC, TransactionKind::Tax, d(2024, 7, 1), dec!(30), "EUR"),
        buy,
    ];

    let holdings = build_holdings(&txns, "EUR", &rates).unwrap();
    let by_year = charges_by_year(&holdings.charges);

    assert_eq!(by_year[&2023].fees_base, dec!(7));
    assert_eq!(by_year[&2023].taxes_base, Decimal::ZERO);
    assert_eq!(by_year[&2023].count, 2);

    assert_eq!(by_year[&2024].fees_base, dec!(18.0));
    assert_eq!(by_year[&2024].taxes_base, dec!(30));
    assert_eq!(
        by_year[&2024].count, 2,
        "a commission inside a buy stays out of the report"
    );
    assert_eq!(by_year[&2024].total_base(), dec!(48.0));

    // Timeline and aggregates must agree.
    let total = charges_total(&holdings.charges);
    assert_eq!(total.fees_base, holdings.fees_base);
    assert_eq!(total.taxes_base, holdings.taxes_base);
}

/// A report window keeps only what falls inside it. Buy 10 × 100 + 5 fee → 1005 cost, 100.5 per share.
/// 2024: sell 4 × 120 = 480 − 3 fee − 2 tax − 402 cost = 73, and 73 / 402 = 0.181592…;
/// dividend 50 − 7.5 tax = 42.5 net; one standalone fee of 12.
/// 2025: sell 2 × 130 = 260 − 1 fee − 201 cost = 58; one standalone tax of 30.
#[test]
fn a_report_window_keeps_only_the_events_inside_it() {
    let txns = vec![
        Transaction::buy(ACC, AAPL, d(2023, 3, 1), dec!(10), dec!(100), "EUR").with_fees(dec!(5)),
        Transaction::sell(ACC, AAPL, d(2024, 5, 1), dec!(4), dec!(120), "EUR")
            .with_fees(dec!(3))
            .with_taxes(dec!(2)),
        Transaction::dividend(ACC, AAPL, d(2024, 6, 1), dec!(50), "EUR").with_taxes(dec!(7.5)),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 8, 1), dec!(12), "EUR"),
        Transaction::sell(ACC, AAPL, d(2025, 2, 1), dec!(2), dec!(130), "EUR").with_fees(dec!(1)),
        Transaction::cash(ACC, TransactionKind::Tax, d(2025, 3, 1), dec!(30), "EUR"),
    ];
    let holdings = build_holdings(&txns, "EUR", &FakeRates::new()).unwrap();

    let year2024 = realized_between(&holdings, d(2024, 1, 1), d(2024, 12, 31));
    let gains2024 = capital_gains_total(&year2024);
    assert_eq!(gains2024.disposals, 1);
    assert_eq!(gains2024.cost_base, dec!(402));
    assert_eq!(gains2024.gain_base, dec!(73));
    assert_eq!(gains2024.return_on_cost().unwrap().round_dp(6), dec!(0.181592));

    assert_eq!(
        dividends_total(&income_between(&holdings, d(2024, 1, 1), d(2024, 12, 31))).net_base,
        dec!(42.5)
    );
    assert_eq!(
        charges_total(&charges_between(&holdings, d(2024, 1, 1), d(2024, 12, 31))).fees_base,
        dec!(12)
    );

    let year2025 = realized_between(&holdings, d(2025, 1, 1), d(2025, 12, 31));
    assert_eq!(capital_gains_total(&year2025).gain_base, dec!(58));
    // A window is not a year: the by-year rollup of one window has exactly that window's years.
    assert_eq!(
        capital_gains_by_year(&year2025).keys().collect::<Vec<_>>(),
        [&2025]
    );

    let charges2025 = charges_between(&holdings, d(2025, 1, 1), d(2025, 12, 31));
    assert_eq!(charges_total(&charges2025).taxes_base, dec!(30));
    assert_eq!(
        charges_total(&charges2025).fees_base,
        Decimal::ZERO,
        "the sell commission sits in the proceeds"
    );

    // Nothing is lost: both windows together are the whole history.
    assert_eq!(
        capital_gains_total(&holdings.realized).gain_base,
        gains2024.gain_base + capital_gains_total(&year2025).gain_base,
    );
}

/// Charges split two ways over the same three events: by account, 12 + 30 = 42 on acc-1
/// and 8 on acc-2; by security, only the 30 EUR tax carries one, so 12 + 8 drop out.
#[test]
fn charges_split_by_account_and_by_security() {
    let mut tax = Transaction::cash(ACC, TransactionKind::Tax, d(2024, 8, 1), dec!(30), "EUR");
    tax.security_id = Some(AAPL.to_string());

    let txns = vec![
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 6, 1), dec!(12), "EUR"),
        Transaction::cash("acc-2", TransactionKind::Fee, d(2024, 7, 1), dec!(8), "EUR"),
        tax,
    ];
    let holdings = build_holdings(&txns, "EUR", &FakeRates::new()).unwrap();

    let by_account = charges_by_account(&holdings.charges);
    assert_eq!(by_account[ACC].total_base(), dec!(42));
    assert_eq!(by_account[ACC].count, 2);
    assert_eq!(by_account["acc-2"].total_base(), dec!(8));

    let by_security = charges_by_security(&holdings.charges);
    assert_eq!(by_security.len(), 1, "an account fee belongs to no security");
    assert_eq!(by_security[AAPL].taxes_base, dec!(30));

    // Every split sums back to the same total: 12 + 8 + 30 = 50.
    let total = charges_total(&holdings.charges);
    assert_eq!(total.total_base(), dec!(50));
    assert_eq!(
        charges_by_kind(&holdings.charges)
            .values()
            .map(|s| s.total_base())
            .sum::<Decimal>(),
        dec!(50),
    );
}

/// The charges report and the cost rate answer different questions and must disagree.
/// A purchase of 10 shares at 100 EUR with a 9.90 commission, a 5.00 standalone fee and a
/// dividend of 40 EUR with 10.60 withheld:
///   charges (standalone only) = 5.00 fees, 0 taxes;
///   costs paid                = 9.90 + 5.00 = 14.90 fees, 10.60 taxes.
#[test]
fn a_cost_rate_counts_the_commission_the_cost_basis_swallowed() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 3, 1), dec!(10), dec!(100), "EUR").with_fees(dec!(9.90)),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 4, 1), dec!(5), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 6, 1), dec!(40), "EUR").with_taxes(dec!(10.60)),
    ];
    let rates = FakeRates::new();
    let holdings = build_holdings(&txs, "EUR", &rates).unwrap();

    let reported = charges_total(&holdings.charges);
    assert_eq!(reported.fees_base, dec!(5));
    assert_eq!(reported.taxes_base, dec!(0));

    let paid = costs_paid(&txs, "EUR", d(2024, 1, 1), d(2024, 12, 31), &rates).unwrap();
    assert_eq!(paid.fees_base, dec!(14.90));
    assert_eq!(paid.taxes_base, dec!(10.60));
    assert_eq!(paid.count, 3);

    // A refund walks the total back: 5.00 - 5.00 leaves only the trade commission.
    let with_refund = [
        txs.clone(),
        vec![Transaction::cash(
            ACC,
            TransactionKind::FeeRefund,
            d(2024, 5, 1),
            dec!(5),
            "EUR",
        )],
    ]
    .concat();
    let refunded = costs_paid(&with_refund, "EUR", d(2024, 1, 1), d(2024, 12, 31), &rates).unwrap();
    assert_eq!(refunded.fees_base, dec!(9.90));
}
