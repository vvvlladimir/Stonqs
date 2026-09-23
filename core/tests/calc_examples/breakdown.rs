use super::*;

/// The calculation sheet of one quarter, worked out by hand.
///
/// Ledger, all in EUR: 01.01 deposit 1000 and buy 10 AAPL at 100; 15.02 dividend 50 gross with
/// 5 tax withheld; 10.03 deposit 200; 20.03 a 10 fee. Prices: 100 on 01.01, 110 on 31.01,
/// 120 on 31.03 — forward-filled in between.
///
/// January  0 + 1000 flow, closes at 10 × 110 = 1100 → earned 100, all of it the market.
/// February 1100 + 0 flow, dividend leaves 45 in cash → closes at 1145, earned 45 = 50 − 5,
///          so the market did nothing.
/// March    1145 + 200 flow, fee takes 10 → cash 45 + 200 − 10 = 235, shares 10 × 120 = 1200,
///          closes at 1435 → earned 90 = 100 of market minus the 10 fee.
///
/// Returns: January 1100/1000 − 1 = 10%; February 1145/1100 − 1 = 45/1100 ≈ 4.0909%;
/// March, whose deposit day earns nothing, 1435/1345 − 1 = 90/1345 ≈ 6.6914%.
/// Chained: 1.10 × 1145/1100 × 1435/1345 = 1145 × 1435 / 1 345 000 ≈ 1.221617, so +22.1617%.
#[test]
fn the_calculation_sheet_adds_up_to_the_period_it_describes() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 2, 15), dec!(50), "EUR").with_taxes(dec!(5)),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 3, 10), dec!(200), "EUR"),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 3, 20), dec!(10), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 1, 1), dec!(100))
        .with(AAPL, d(2024, 1, 31), dec!(110))
        .with(AAPL, d(2024, 3, 31), dec!(120));
    let rates = FakeRates::new();

    let range = DateRange::new(d(2024, 1, 1), d(2024, 3, 31));
    let series = value_series(&txs, "EUR", range, &prices, &rates, HoldingsOptions::default()).unwrap();
    let holdings = build_holdings(&txs, "EUR", &rates).unwrap();
    let sheet = calculation_sheet(&series, &holdings, &txs, &rates, Period::Month).unwrap();

    assert_eq!(sheet.rows.len(), 3);

    let january = &sheet.rows[0];
    assert_eq!(january.start_value_base, dec!(0));
    assert_eq!(january.external_flow_base, dec!(1000));
    assert_eq!(january.end_value_base, dec!(1100));
    assert_eq!(january.delta_base, dec!(100));
    assert_eq!(january.market_change_base, dec!(100));
    assert_eq!(january.twr.round_dp(6), dec!(0.100000));

    let february = &sheet.rows[1];
    assert_eq!(february.start_value_base, dec!(1100));
    assert_eq!(february.external_flow_base, dec!(0));
    assert_eq!(february.income_base, dec!(50));
    assert_eq!(february.costs.taxes_base, dec!(5));
    assert_eq!(february.delta_base, dec!(45));
    // 45 − 50 + 5: the month's whole change was the payment, so the market did nothing.
    assert_eq!(february.market_change_base, dec!(0));
    assert_eq!(february.twr.round_dp(6), dec!(0.040909));

    let march = &sheet.rows[2];
    assert_eq!(march.start_value_base, dec!(1145));
    assert_eq!(march.external_flow_base, dec!(200));
    assert_eq!(march.costs.fees_base, dec!(10));
    assert_eq!(march.end_value_base, dec!(1435));
    assert_eq!(march.delta_base, dec!(90));
    assert_eq!(march.market_change_base, dec!(100));
    assert_eq!(march.twr.round_dp(6), dec!(0.066914));

    // Every row closes where the next one opens, so the rows telescope into the totals.
    for pair in sheet.rows.windows(2) {
        assert_eq!(pair[0].end_value_base, pair[1].start_value_base);
    }
    let flows: Decimal = sheet.rows.iter().map(|r| r.external_flow_base).sum();
    let earned: Decimal = sheet.rows.iter().map(|r| r.delta_base).sum();
    assert_eq!(flows, sheet.total.net_flow_base);
    assert_eq!(earned, sheet.total.delta_base);
    assert_eq!(sheet.rows[2].end_value_base, sheet.total.end_value_base);
    assert_eq!(sheet.rows[0].start_value_base, sheet.total.start_value_base);

    // The whole point of the sheet: the rows chain to the very TWR shown beside them.
    assert_eq!(sheet.twr.round_dp(6), dec!(0.221617));
    assert_eq!(
        march.cumulative_twr.round_dp(10),
        sheet.twr.round_dp(10),
        "the last row's chained return is the period's own"
    );
    // And that period return is the one the metric strip shows, which is computed the other
    // way round — split at each flow rather than chained daily. The sheet is only worth
    // showing while the two agree.
    let headline = twr_between(&txs, "EUR", d(2024, 1, 1), d(2024, 3, 31), &prices, &rates).unwrap();
    assert_eq!(headline.round_dp(10), sheet.twr.round_dp(10));
}

/// Each row's change is exactly what it is made of: market plus income minus costs.
#[test]
fn every_row_splits_its_earnings_into_market_income_and_costs() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 2, 15), dec!(50), "EUR").with_taxes(dec!(5)),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 3, 20), dec!(10), "EUR"),
    ];
    let prices =
        FakePrices::new("EUR")
            .with(AAPL, d(2024, 1, 1), dec!(100))
            .with(AAPL, d(2024, 3, 31), dec!(120));
    let rates = FakeRates::new();

    let range = DateRange::new(d(2024, 1, 1), d(2024, 3, 31));
    let series = value_series(&txs, "EUR", range, &prices, &rates, HoldingsOptions::default()).unwrap();
    let holdings = build_holdings(&txs, "EUR", &rates).unwrap();
    let sheet = calculation_sheet(&series, &holdings, &txs, &rates, Period::Month).unwrap();

    for row in &sheet.rows {
        assert_eq!(
            row.delta_base,
            row.market_change_base + row.income_base - row.costs.total_base(),
            "row {} does not account for its own change",
            row.from
        );
        assert_eq!(
            row.end_value_base,
            row.start_value_base + row.external_flow_base + row.delta_base,
            "row {} does not close where its parts put it",
            row.from
        );
    }
}
