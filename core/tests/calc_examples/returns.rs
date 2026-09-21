use super::*;

/// TWR scenario: 01.06 = 920 EUR, 05.06 cost = 897 EUR, 01.07 value = 967.50 EUR;
/// TWR = 967.50 / 920 − 1 ≈ +5.163%.
#[test]
fn twr_over_a_full_scenario() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 1), dec!(1000), "USD")
            .with_fx_rate(dec!(0.92)),
        Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD").with_fx_rate(dec!(0.92)),
    ];

    let prices =
        FakePrices::new("USD")
            .with(AAPL, d(2024, 6, 5), dec!(195))
            .with(AAPL, d(2024, 7, 1), dec!(210));
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 1), dec!(0.92))
        .with("USD", "EUR", d(2024, 7, 1), dec!(0.90));

    let start = valuation_at(&txs, "EUR", d(2024, 6, 1), &prices, &rates).unwrap();
    assert_eq!(start.total_value_base, dec!(920.00));

    let end = valuation_at(&txs, "EUR", d(2024, 7, 1), &prices, &rates).unwrap();
    assert_eq!(end.securities_value_base, dec!(945.0000));
    assert_eq!(end.cash_base, dec!(22.500));
    assert_eq!(end.total_value_base, dec!(967.5000));

    let twr = twr_between(&txs, "EUR", d(2024, 6, 1), d(2024, 7, 1), &prices, &rates).unwrap();
    assert_eq!(twr.round_dp(6), dec!(0.051630));
}

/// A mid-period deposit is not return: r₁ = 10%, r₂ = 11%, TWR = 22.10%;
/// simple return is 16.8%.
#[test]
fn twr_splits_the_period_at_the_cash_flow() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 7, 1), dec!(900), "EUR"),
    ];

    // This scenario uses EUR for trades and quotes.
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 1, 1), dec!(100))
        .with(AAPL, d(2024, 6, 30), dec!(110))
        .with(AAPL, d(2024, 12, 31), dec!(132));
    let rates = FakeRates::new();

    let mid = valuation_at(&txs, "EUR", d(2024, 6, 30), &prices, &rates).unwrap();
    assert_eq!(mid.total_value_base, dec!(1100));
    let end = valuation_at(&txs, "EUR", d(2024, 12, 31), &prices, &rates).unwrap();
    assert_eq!(end.total_value_base, dec!(2220));

    let twr = twr_between(&txs, "EUR", d(2024, 1, 1), d(2024, 12, 31), &prices, &rates).unwrap();
    assert_eq!(twr.round_dp(10), dec!(0.2210));
}

/// XIRR = (967.50 / 920.00)^(365/30) − 1 ≈ +84.5% annualized;
/// the monthly return remains +5.16%.
#[test]
fn xirr_annualizes_a_one_month_gain() {
    let flows = vec![
        CashFlow {
            date: d(2024, 6, 1),
            amount_base: dec!(-920.00),
        },
        CashFlow {
            date: d(2024, 7, 1),
            amount_base: dec!(967.50),
        },
    ];
    let r = xirr(&flows).unwrap();

    let expected = (967.5_f64 / 920.0).powf(365.0 / 30.0) - 1.0;
    assert!(
        (r.to_f64().unwrap() - expected).abs() < 1e-6,
        "got {r}, expected ≈ {expected}"
    );
    assert_eq!(r.round_dp(3), dec!(0.845));
}

/// Period contributions: A gain = 350 EUR, B gain = −50 EUR;
/// TWR A = 26.1765%, TWR B = −5.2632%, portfolio return = 3300/3000 − 1 = 10%.
#[test]
fn position_returns_split_the_portfolio_result() {
    const A: &str = "sec-a";
    const B: &str = "sec-b";

    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(3000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::buy(ACC, B, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::buy(ACC, A, d(2024, 7, 1), dec!(5), dec!(120), "EUR"),
        Transaction::dividend(ACC, B, d(2024, 9, 1), dec!(50), "EUR"),
    ];

    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 1, 1), dec!(100))
        .with(A, d(2024, 6, 30), dec!(110))
        .with(A, d(2024, 12, 31), dec!(130))
        .with(B, d(2024, 1, 1), dec!(100))
        .with(B, d(2024, 8, 31), dec!(100))
        .with(B, d(2024, 12, 31), dec!(90));
    let rates = FakeRates::new();

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();

    assert_eq!(rows.len(), 2);
    let a = rows.iter().find(|r| r.security_id == A).unwrap();
    let b = rows.iter().find(|r| r.security_id == B).unwrap();

    assert_eq!(a.pnl_base, dec!(350));
    assert_eq!(b.pnl_base, dec!(-50));
    assert_eq!(a.twr.unwrap().round_dp(6), dec!(0.261765));
    assert_eq!(b.twr.unwrap().round_dp(6), dec!(-0.052632));
    assert_eq!(a.contribution.round_dp(6), dec!(0.116667));
    assert_eq!(b.contribution.round_dp(6), dec!(-0.016667));

    // Both positions have opposite-signed flows, so IRR is defined.
    assert!(a.xirr.is_some() && b.xirr.is_some());

    let portfolio = twr_between(&txs, "EUR", d(2024, 1, 1), d(2024, 12, 31), &prices, &rates).unwrap();
    assert_eq!(portfolio, dec!(0.1));
    let contributions: Decimal = rows.iter().map(|r| r.contribution).sum();
    assert_eq!(contributions.round_dp(6), dec!(0.1));
}

/// Contribution divides by the capital at work, not by the opening value. Period 2024
/// (365 days): opens at 1 EUR, 999 EUR arrive on 31 January and stay 335 days, so the
/// capital is 1 + 999 × 335/365 = 917.8904 EUR. The position gains 10 × (120 − 100) = 200,
/// which is +21.789% of that — against the opening euro alone it would read +20 000%.
#[test]
fn contribution_divides_by_the_capital_at_work() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1), "EUR"),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 31), dec!(999), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 2, 1), dec!(10), dec!(100), "EUR"),
    ];
    let prices =
        FakePrices::new("EUR")
            .with(AAPL, d(2024, 2, 1), dec!(100))
            .with(AAPL, d(2024, 12, 31), dec!(120));

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &prices,
        &FakeRates::new(),
        HoldingsOptions::default(),
    )
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pnl_base, dec!(200));
    assert_eq!(rows[0].contribution.round_dp(6), dec!(0.217891));
}

/// A position closed before the period is omitted from the table.
#[test]
fn closed_positions_stay_out_of_the_period() {
    const OLD: &str = "sec-old";

    let txs = vec![
        Transaction::buy(ACC, OLD, d(2023, 1, 10), dec!(10), dec!(100), "EUR"),
        Transaction::sell(ACC, OLD, d(2023, 6, 10), dec!(10), dec!(120), "EUR"),
        // Proceeds leave the account, so the period starts empty.
        Transaction::cash(
            ACC,
            TransactionKind::Withdrawal,
            d(2023, 12, 31),
            dec!(200),
            "EUR",
        ),
        Transaction::buy(ACC, AAPL, d(2024, 3, 1), dec!(10), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(OLD, d(2023, 1, 10), dec!(100))
        .with(AAPL, d(2024, 3, 1), dec!(100))
        .with(AAPL, d(2024, 12, 31), dec!(120));

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &prices,
        &FakeRates::new(),
        HoldingsOptions::default(),
    )
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].security_id, AAPL);
    // Bought in-period: 10 × 100 = 1000 invested, ending value 10 × 120 = 1200.
    assert_eq!(rows[0].pnl_base, dec!(200));
    // Empty opening portfolio means no share, not zero return.
    assert_eq!(rows[0].contribution, Decimal::ZERO);
}

/// Daily change follows quotes, not calendar days: Thursday 100 → Friday 110 gives
/// (110 − 100) × 10 = 100 EUR (+10%); Saturday still compares with Thursday.
#[test]
fn day_change_steps_back_along_the_quotes_not_the_calendar() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 6, 6),
        dec!(10),
        dec!(100),
        "EUR",
    )];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 6, 6), dec!(100)) // Thursday
        .with(AAPL, d(2024, 6, 7), dec!(110)); // Friday
    let rates = FakeRates::new();

    let friday = day_changes(&holdings, "EUR", d(2024, 6, 7), &prices, &rates).unwrap();
    let change = &friday.positions[AAPL];
    assert_eq!(change.previous_price, dec!(100));
    assert_eq!(change.change_base, dec!(100));
    assert_eq!(change.change, dec!(0.1));
    assert_eq!(friday.total_base, dec!(100));

    let saturday = day_changes(&holdings, "EUR", d(2024, 6, 8), &prices, &rates).unwrap();
    assert_eq!(saturday.positions[AAPL].change_base, dec!(100));

    let thursday = day_changes(&holdings, "EUR", d(2024, 6, 6), &prices, &rates).unwrap();
    assert!(thursday.positions.is_empty());
    assert_eq!(thursday.total_base, Decimal::ZERO);
}

/// USD daily change is price movement, not FX movement: (210 − 200) × 10 × 0.90 = 90 EUR.
#[test]
fn day_change_converts_both_prices_by_the_same_rate() {
    let txs =
        vec![Transaction::buy(ACC, AAPL, d(2024, 6, 6), dec!(10), dec!(200), "USD").with_fx_rate(dec!(0.92))];
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 6), dec!(0.92))
        .with("USD", "EUR", d(2024, 6, 7), dec!(0.90));
    let holdings = build_holdings(&txs, "EUR", &rates).unwrap();
    let prices =
        FakePrices::new("USD")
            .with(AAPL, d(2024, 6, 6), dec!(200))
            .with(AAPL, d(2024, 6, 7), dec!(210));

    let changes = day_changes(&holdings, "EUR", d(2024, 6, 7), &prices, &rates).unwrap();
    assert_eq!(changes.positions[AAPL].change_base, dec!(90.00));
    assert_eq!(changes.positions[AAPL].change.round_dp(6), dec!(0.05));
}

/// A deposit is not a result. Start 2024-01-01 with 1000 EUR of cash, buy 10 shares at 100;
/// on 2024-07-01 deposit 500 and buy 5 more at 100; the price closes 2024-12-31 at 120.
///
/// The window opens on what was there before it, and on 2024-01-01 that was nothing: the day's
/// own 1000 is money paid in, so the opening balance is 1000 - 1000 = 0.
///
/// End value  = 15 * 120 = 1800.
/// Net flow = 1000 + 500 = 1500.
/// Absolute change = 1800 - 0 = 1800, of which 1500 was brought in;
/// delta = 1800 - 1500 = 300 (10 shares gained 200, 5 shares gained 100).
/// Invested capital = 0 + 1500 = 1500.
/// Average capital measures from the first day's value instead: the 1000 arrived on the opening
/// day and worked the whole window, exactly as Dietz weights every later flow.
/// Average capital  = 1000 + 500 * (2024-12-31 - 2024-07-01) / (2024-12-31 - 2024-01-01)
///                  = 1000 + 500 * 183 / 365 = 1250.684931506849315068...
#[test]
fn a_period_summary_separates_earnings_from_deposits() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 7, 1), dec!(500), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 7, 1), dec!(5), dec!(100), "EUR"),
    ];
    let prices =
        FakePrices::new("EUR")
            .with(AAPL, d(2024, 1, 1), dec!(100))
            .with(AAPL, d(2024, 12, 31), dec!(120));
    let rates = FakeRates::new();

    let series = value_series(
        &txs,
        "EUR",
        DateRange::new(d(2024, 1, 1), d(2024, 12, 31)),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();
    let summary = period_summary(&series);

    assert_eq!(summary.start_value_base, Decimal::ZERO);
    assert_eq!(summary.end_value_base, dec!(1800));
    // Both deposits count: neither of them was already there when the window opened.
    assert_eq!(summary.net_flow_base, dec!(1500));
    assert_eq!(summary.absolute_change_base, dec!(1800));
    assert_eq!(summary.delta_base, dec!(300));
    assert_eq!(summary.invested_capital_base, dec!(1500));
    assert_eq!(summary.average_capital_base.round_dp(6), dec!(1250.684932));

    // A 12.51 fee on that capital is 12.51 / 1250.6849... = 1.000252 %;
    // the rate divides in the core, never in the UI.
    assert_eq!(summary.rate_of(dec!(12.51)).unwrap().round_dp(6), dec!(0.010003));
}

/// Contribution and absolute performance divide by different denominators on purpose.
/// One position: 10 shares at 100 on 2024-01-01, 5 more at 120 on 2024-07-01, price 130 at
/// year end, in a portfolio that also holds 1400 EUR of idle cash.
///
/// Position P/L    = 15 * 130 - 1000 - 600 = 350.
/// Position capital  = 1000 + 600 * 183 / 365 = 1300.8219178...
///   -> absolute performance = 350 / 1300.8219... = 0.2690606...
/// Portfolio capital = 2400 + 600 * 183 / 365 = 2700.8219178...
///   -> contribution         = 350 / 2700.8219... = 0.1295901...
#[test]
fn absolute_performance_divides_by_the_positions_own_capital() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(2400), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 7, 1), dec!(600), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 7, 1), dec!(5), dec!(120), "EUR"),
    ];
    let prices =
        FakePrices::new("EUR")
            .with(AAPL, d(2024, 1, 1), dec!(100))
            .with(AAPL, d(2024, 12, 31), dec!(130));
    let rates = FakeRates::new();

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();
    let row = rows.iter().find(|r| r.security_id == AAPL).unwrap();

    assert_eq!(row.pnl_base, dec!(350));
    assert_eq!(row.absolute_performance.unwrap().round_dp(6), dec!(0.269061));
    assert_eq!(row.contribution.round_dp(6), dec!(0.129590));

    // The period is exactly 365 days, so the yearly rate is the period rate.
    assert_eq!(
        row.twr_annualized.unwrap().round_dp(6),
        row.twr.unwrap().round_dp(6)
    );
}

/// Max drawdown and max drawdown duration need not be the same episode, and the drawdown
/// standing right now is not the depth of the hole it is climbing out of.
///
/// Business-day values 100, 80, 100, 90, 91, 92, 93 over 2024-06-03..2024-06-11. The chained
/// index starts at the *second* value, so the first episode's peak is 2024-06-04, the day the
/// fall is first seen:
///   -20% on the 4th, back to the peak on the 5th   -> closed, 1 day, depth -0.20;
///   -10% on the 6th, never back to the peak        -> open to the 11th, 6 days, depth -0.10.
/// The deepest is the first, the longest is the second, and by the 11th the index is
/// 0.8 * 1.25 * 0.9 * (91/90) * (92/91) * (93/92) = 0.93, so the standing drawdown is -7%.
#[test]
fn drawdown_duration_counts_to_the_series_end_while_still_underwater() {
    let values = [
        (d(2024, 6, 3), dec!(100)),
        (d(2024, 6, 4), dec!(80)),
        (d(2024, 6, 5), dec!(100)),
        (d(2024, 6, 6), dec!(90)),
        (d(2024, 6, 7), dec!(91)),
        (d(2024, 6, 10), dec!(92)),
        (d(2024, 6, 11), dec!(93)),
    ];
    let series = sq_core::calc::ValueSeries {
        base_currency: "EUR".into(),
        dates: values.iter().map(|(date, _)| *date).collect(),
        total_value_base: values.iter().map(|(_, v)| *v).collect(),
        external_flow_base: values.iter().map(|_| Decimal::ZERO).collect(),
    };

    let metrics = risk_metrics(&series, 0.0);

    let deepest = metrics.max_drawdown.as_ref().unwrap();
    assert_eq!(deepest.peak, d(2024, 6, 4));
    assert_eq!(deepest.recovered, Some(d(2024, 6, 5)));
    assert!(
        (deepest.depth - (-0.20)).abs() < 1e-12,
        "depth = {}",
        deepest.depth
    );
    assert_eq!(metrics.max_drawdown_days, Some(1));

    let longest = metrics.longest_drawdown.as_ref().unwrap();
    assert_eq!(longest.peak, d(2024, 6, 5));
    assert_eq!(longest.recovered, None);
    assert!(
        (longest.depth - (-0.10)).abs() < 1e-12,
        "depth = {}",
        longest.depth
    );
    assert_eq!(metrics.longest_drawdown_days, Some(6));

    // Still underwater, but 3 points above the trough the table reports.
    assert_eq!(metrics.current_drawdown_since, Some(d(2024, 6, 5)));
    assert!(
        (metrics.current_drawdown - (-0.07)).abs() < 1e-12,
        "current = {}",
        metrics.current_drawdown
    );
}

/// Per-position risk, one week of 10 shares bought Monday 2024-06-03 at 100.
///
/// Closes Tue 110, Wed 99, Thu 94.05, Fri 112.86 → daily returns +10%, −10%, −5%, +20%.
/// Chained: 1.1 × 0.9 × 0.95 × 1.2 = 1.1286, the position's TWR of 12.86%.
/// Mean 0.0375; squared deviations 0.0625², 0.1375², 0.0875², 0.1625² sum to 0.056875;
/// sample variance 0.056875 / 3; volatility = √(0.056875 / 3) × √252.
/// Downside days −0.10 and −0.05, over every day rather than only the losing ones, so the
/// denominator is the volatility's: (0.01 + 0.0025) / 3; semi-deviation = √(0.0125 / 3) × √252.
/// Deepest drawdown: peak Tue 110 to trough Thu 94.05 = −14.5%, recovered Fri — 3 days.
#[test]
fn position_risk_reads_the_same_daily_returns_its_twr_chains() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 6, 3),
        dec!(10),
        dec!(100),
        "EUR",
    )];
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 6, 3), dec!(100))
        .with(AAPL, d(2024, 6, 4), dec!(110))
        .with(AAPL, d(2024, 6, 5), dec!(99))
        .with(AAPL, d(2024, 6, 6), dec!(94.05))
        .with(AAPL, d(2024, 6, 7), dec!(112.86));

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 6, 3),
        d(2024, 6, 7),
        &prices,
        &FakeRates::new(),
        HoldingsOptions::default(),
    )
    .unwrap();

    let row = &rows[0];
    assert_eq!(row.twr.unwrap(), dec!(0.1286));
    let risk = row.risk.as_ref().unwrap();
    let annual = 252f64.sqrt();
    assert!((risk.volatility - (0.056875f64 / 3.0).sqrt() * annual).abs() < 1e-9);
    assert!((risk.semi_deviation - (0.0125f64 / 3.0).sqrt() * annual).abs() < 1e-9);
    assert!((risk.max_drawdown - (-0.145)).abs() < 1e-9);
    assert_eq!(risk.max_drawdown_days, Some(3));
}

/// A position sold off mid-period ends its returns on the sale: 10 shares at 100, Tuesday close
/// 110, sold Wednesday for 1 150 (115 each). Tuesday +10%; Wednesday is what came out over what
/// was there, 1 150 / 1 100 − 1 = +4.545%, not −100% of a position that is simply gone.
/// Thursday and Friday nothing is held, so there is no return to count.
#[test]
fn position_risk_reads_a_sale_as_money_out_not_a_crash() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 6, 3), dec!(10), dec!(100), "EUR"),
        Transaction::sell(ACC, AAPL, d(2024, 6, 5), dec!(10), dec!(115), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(AAPL, d(2024, 6, 3), dec!(100))
        .with(AAPL, d(2024, 6, 4), dec!(110))
        .with(AAPL, d(2024, 6, 5), dec!(114));

    let rows = position_returns(
        &txs,
        "EUR",
        d(2024, 6, 3),
        d(2024, 6, 7),
        &prices,
        &FakeRates::new(),
        HoldingsOptions::default(),
    )
    .unwrap();

    let risk = rows[0].risk.as_ref().unwrap();
    assert_eq!(risk.max_drawdown, 0.0, "the position never fell");
    assert_eq!(risk.semi_deviation, 0.0);
    // Two returns, +0.10 and +0.0454545…: sample deviation |0.1 − 0.0454545| / √2.
    let expected = (0.1f64 - 50.0 / 1100.0).abs() / 2f64.sqrt() * 252f64.sqrt();
    assert!((risk.volatility - expected).abs() < 1e-9);
}
