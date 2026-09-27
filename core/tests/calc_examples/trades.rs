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
    let trades = closed_trades(&holdings.realized, TradeGrouping::Position);

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
    let trades = open_trades(&holdings, &valuation, TradeGrouping::Position);

    assert_eq!(trades.len(), 1);
    assert!(trades[0].is_open());
    assert_eq!(trades[0].entry_value_base, dec!(500));
    assert_eq!(trades[0].exit_value_base, dec!(750));
    assert_eq!(trades[0].pnl_base, dec!(250));
    assert_eq!(trades[0].holding_days, 120);
}

/// The same two purchases and one sale, cut per lot: two trades rather than one.
/// Lot A cost 10 × 100 + 5 = 1005, lot B 10 × 120 + 5 = 1205.
/// The sale's 2594 net is shared by quantity, 10 of 20 each: 2594 × 10/20 = 1297 per lot.
/// A: 1297 − 1005 = 292 over 366 days. B: 1297 − 1205 = 92 over 184 days.
/// Together 292 + 92 = 384 — the per-position result, split rather than changed.
#[test]
fn per_lot_one_sale_of_two_purchases_is_two_trades() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR").with_fees(dec!(5)),
        Transaction::buy(ACC, AAPL, d(2024, 7, 1), dec!(10), dec!(120), "EUR").with_fees(dec!(5)),
        Transaction::sell(ACC, AAPL, d(2025, 1, 1), dec!(20), dec!(130), "EUR").with_fees(dec!(6)),
    ];

    let holdings = build_holdings(&transactions, "EUR", &FakeRates::new()).unwrap();
    let trades = closed_trades(&holdings.realized, TradeGrouping::Lot);

    assert_eq!(trades.len(), 2);
    let (a, b) = (&trades[0], &trades[1]);
    assert_eq!((a.opened_at, a.quantity), (d(2024, 1, 1), dec!(10)));
    assert_eq!(a.entry_value_base, dec!(1005));
    assert_eq!(a.exit_value_base, dec!(1297));
    assert_eq!(a.pnl_base, dec!(292));
    assert_eq!(a.holding_days, 366);
    assert_eq!((b.opened_at, b.quantity), (d(2024, 7, 1), dec!(10)));
    assert_eq!(b.entry_value_base, dec!(1205));
    assert_eq!(b.exit_value_base, dec!(1297));
    assert_eq!(b.pnl_base, dec!(92));
    assert_eq!(b.holding_days, 184);
    assert_eq!(a.closed_at, b.closed_at);

    let whole = closed_trades(&holdings.realized, TradeGrouping::Position);
    assert_eq!(a.pnl_base + b.pnl_base, whole[0].pnl_base);
}

/// A sale that empties one lot and cuts into the next, first in first out.
/// Bought 10 at 100 and 10 at 120; sold 15 at 130, so 1950 came in and FIFO took all 10 of the
/// first lot and 5 of the second. Shared by quantity: 1950 × 10/15 = 1300 and 1950 × 5/15 = 650.
/// Closed: 1300 − 1000 = 300, and 650 − 5 × 120 = 650 − 600 = 50.
/// Still open: the second lot's other 5, worth 5 × 150 = 750 against 600, so 150 unrealized.
#[test]
fn per_lot_a_partial_sale_closes_what_it_consumed_and_leaves_the_rest_open() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 7, 1), dec!(10), dec!(120), "EUR"),
        Transaction::sell(ACC, AAPL, d(2025, 1, 1), dec!(15), dec!(130), "EUR"),
    ];
    let prices = FakePrices::new("EUR").with(AAPL, d(2025, 6, 1), dec!(150));
    let rates = FakeRates::new();

    let holdings = build_holdings(&transactions, "EUR", &rates).unwrap();
    let closed = closed_trades(&holdings.realized, TradeGrouping::Lot);
    assert_eq!(closed.len(), 2);
    assert_eq!((closed[0].quantity, closed[0].pnl_base), (dec!(10), dec!(300)));
    assert_eq!((closed[1].quantity, closed[1].pnl_base), (dec!(5), dec!(50)));

    let valuation = value_holdings(&holdings, "EUR", d(2025, 6, 1), &prices, &rates).unwrap();
    let open = open_trades(&holdings, &valuation, TradeGrouping::Lot);
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].opened_at, d(2024, 7, 1));
    assert_eq!(open[0].quantity, dec!(5));
    assert_eq!(open[0].entry_value_base, dec!(600));
    assert_eq!(open[0].exit_value_base, dec!(750));
    assert_eq!(open[0].pnl_base, dec!(150));
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
