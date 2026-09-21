use super::*;

/// Two purchases and one sale are one closed trade.
/// Entry: 10 × 100 + 5 fee = 1005, then 10 × 120 + 5 = 1205, so 2210 went in.
/// Exit: 20 × 130 − 6 fee = 2594 came out, a result of 2594 − 2210 = 384.
/// Return = 2594 / 2210 − 1 = 0.173756.
/// Ages on the sale date are 366 and 184 days, weighted (10×366 + 10×184) / 20 = 275.
#[test]
fn two_purchases_and_a_sale_make_one_closed_trade() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR").with_fees(dec!(5)),
        Transaction::buy(ACC, AAPL, d(2024, 7, 1), dec!(10), dec!(120), "EUR").with_fees(dec!(5)),
        Transaction::sell(ACC, AAPL, d(2025, 1, 1), dec!(20), dec!(130), "EUR").with_fees(dec!(6)),
    ];

    let holdings = build_holdings(&transactions, "EUR", &FakeRates::new()).unwrap();
    let trades = closed_trades(&holdings.realized);

    assert_eq!(trades.len(), 1);
    let trade = &trades[0];
    assert_eq!(trade.opened_at, d(2024, 1, 1));
    assert_eq!(trade.closed_at, Some(d(2025, 1, 1)));
    assert_eq!(trade.quantity, dec!(20));
    assert_eq!(trade.entry_value_base, dec!(2210));
    assert_eq!(trade.exit_value_base, dec!(2594));
    assert_eq!(trade.pnl_base, dec!(384));
    assert_eq!(trade.holding_days, 275);
    assert_eq!(trade.return_pct.unwrap().round_dp(6), dec!(0.173756));

    let stats = trade_stats(&trades);
    assert_eq!(stats.trades, 1);
    assert_eq!(stats.winners, 1);
    assert_eq!(stats.pnl_base, dec!(384));
}

/// A position still held is a trade marked to market: 5 × 100 = 500 in,
/// 5 × 150 = 750 now, so 250 unrealized and no closing date.
#[test]
fn an_open_position_is_an_open_trade() {
    let buy = Transaction::buy(ACC, AAPL, d(2025, 2, 1), dec!(5), dec!(100), "EUR");
    let prices = FakePrices::new("EUR").with(AAPL, d(2025, 6, 1), dec!(150));
    let rates = FakeRates::new();

    let holdings = build_holdings(&[buy], "EUR", &rates).unwrap();
    let valuation = value_holdings(&holdings, "EUR", d(2025, 6, 1), &prices, &rates).unwrap();
    let trades = open_trades(&holdings, &valuation);

    assert_eq!(trades.len(), 1);
    assert!(trades[0].is_open());
    assert_eq!(trades[0].entry_value_base, dec!(500));
    assert_eq!(trades[0].exit_value_base, dec!(750));
    assert_eq!(trades[0].pnl_base, dec!(250));
    assert_eq!(trades[0].holding_days, 120);
}

/// Turnover counts the trading itself: 1000 bought and 600 sold move 1600,
/// while shares delivered in from another broker were nobody's decision here.
#[test]
fn turnover_counts_trades_and_ignores_deliveries() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 3, 1), dec!(10), dec!(100), "EUR"),
        Transaction::sell(ACC, AAPL, d(2024, 5, 1), dec!(5), dec!(120), "EUR"),
        Transaction::delivery_inbound(ACC, AAPL, d(2024, 4, 1), dec!(3), dec!(100), "EUR"),
        // Outside the window: a trade of the year before is not this period's turnover.
        Transaction::buy(ACC, AAPL, d(2023, 3, 1), dec!(10), dec!(100), "EUR"),
    ];

    let volume = trading_volume(
        &transactions,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &FakeRates::new(),
    )
    .unwrap();

    assert_eq!(volume.bought_base, dec!(1000));
    assert_eq!(volume.sold_base, dec!(600));
    assert_eq!(volume.volume_base, dec!(1600));
    assert_eq!(volume.trades, 2);
}

/// 1000 deposited and spent on 10 shares; the price runs 100 → 120 → 90.
/// The high is 1200 on the second day, and the last day stands
/// 900 / 1200 − 1 = −0.25 below it, two days later.
#[test]
fn the_all_time_high_is_the_best_day_the_statement_had() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 6, 1), dec!(10), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 6, 1), dec!(100))
        .with(AAPL, d(2024, 6, 2), dec!(120))
        .with(AAPL, d(2024, 6, 3), dec!(90));
    let rates = FakeRates::new();

    let series = value_series(
        &transactions,
        "EUR",
        DateRange::new(d(2024, 6, 1), d(2024, 6, 3)),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();
    let peak = all_time_high(&series).unwrap();

    assert_eq!(peak.date, d(2024, 6, 2));
    assert_eq!(peak.value, dec!(1200));
    assert_eq!(peak.current, dec!(900));
    assert_eq!(peak.distance, Some(dec!(-0.25)));
    assert_eq!(peak.days_since, 1);
}
