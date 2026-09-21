use super::*;

/// Cost basis uses the trade's actual FX rate: gross = 975 USD, rate ≈ 0.923076923,
/// and cost = 900.00 EUR (the reference rate is 0.80).
#[test]
fn cost_basis_uses_the_rate_actually_paid() {
    let buy = Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD")
        .with_fx_from_base_total(dec!(900));

    let rates = FakeRates::new().with("USD", "EUR", d(2024, 6, 5), dec!(0.80));
    let holdings = build_holdings(&[buy], "EUR", &rates).unwrap();
    let position = &holdings.positions[AAPL];

    assert_eq!(position.quantity, dec!(5));
    assert_eq!(position.cost_basis, dec!(975)); // security currency
    assert_eq!(position.cost_basis_base.round_dp(2), dec!(900.00)); // not 780
}

/// Settlement and listing currencies can differ: buy = 1000 EUR, value = 1170 EUR,
/// unrealized P/L = 1170 − 1000 = 170 EUR.
#[test]
fn a_dollar_listing_bought_in_euro_is_converted_by_the_quote_currency() {
    let buy = Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(10), dec!(100), "EUR");

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 7, 1), dec!(130));
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 7, 1), dec!(0.90));

    let holdings = build_holdings(&[buy], "EUR", &rates).unwrap();
    assert_eq!(holdings.positions[AAPL].cost_currency, "EUR");

    let v = value_holdings(&holdings, "EUR", d(2024, 7, 1), &prices, &rates).unwrap();
    let p = &v.positions[0];

    assert_eq!(p.currency, "USD", "the price is signed with the quote currency");
    assert_eq!(
        p.cost_currency, "EUR",
        "the cost basis carries the settlement currency"
    );
    assert_eq!(p.fx_rate, dec!(0.90), "the rate is taken from the quote currency");
    assert_eq!(p.market_value, dec!(1300));
    assert_eq!(p.market_value_base, dec!(1170.00));
    assert_eq!(p.unrealized_pnl_base, dec!(170.00));
}

/// Base-currency valuation separates security and FX effects: cost = 897 EUR,
/// value = 945 EUR, unrealized P/L = 48 EUR (USD price gain alone is 67.50 EUR).
#[test]
fn position_valuation_across_currencies() {
    let buy = Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD").with_fx_rate(dec!(0.92));

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 7, 1), dec!(210));
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 5), dec!(0.92))
        .with("USD", "EUR", d(2024, 7, 1), dec!(0.90));

    let holdings = build_holdings(&[buy], "EUR", &rates).unwrap();
    let v = value_holdings(&holdings, "EUR", d(2024, 7, 1), &prices, &rates).unwrap();

    assert_eq!(v.positions.len(), 1);
    let p = &v.positions[0];
    assert_eq!(p.market_value, dec!(1050.00));
    assert_eq!(p.market_value_base, dec!(945.0000));
    assert_eq!(p.cost_basis_base, dec!(897.00));
    assert_eq!(p.unrealized_pnl_base, dec!(48.0000));

    // Current FX does not retroactively revalue cost basis.
    assert_eq!(v.cost_basis_base, dec!(897.00));
}

/// FIFO partial sale: proceeds = 2240, cost = 1600, realized P/L = 640;
/// remainder cost = 600, cash = 40, ending value = 840.
#[test]
fn fifo_realized_and_unrealized_pnl() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD"),
        Transaction::buy(ACC, AAPL, d(2024, 2, 10), dec!(10), dec!(120), "USD"),
        Transaction::sell(ACC, AAPL, d(2024, 3, 10), dec!(15), dec!(150), "USD").with_fees(dec!(10)),
    ];

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 4, 1), dec!(160));
    let rates = FakeRates::new();

    let holdings = build_holdings(&txs, "USD", &rates).unwrap();
    let p = &holdings.positions[AAPL];
    assert_eq!(p.quantity, dec!(5));
    assert_eq!(p.cost_basis_base, dec!(600));
    assert_eq!(p.realized_pnl_base, dec!(640));
    assert_eq!(
        p.lots.len(),
        1,
        "the first lot is sold out; only the second is left"
    );
    assert_eq!(p.lots[0].cost_per_unit, dec!(120));
    assert_eq!(holdings.cash["USD"], dec!(40));

    let v = value_holdings(&holdings, "USD", d(2024, 4, 1), &prices, &rates).unwrap();
    assert_eq!(v.securities_value_base, dec!(800));
    assert_eq!(v.unrealized_pnl_base, dec!(200));
    assert_eq!(v.cash_base, dec!(40));
    assert_eq!(v.total_value_base, dec!(840));
    assert_eq!(v.realized_pnl_base, dec!(640));
}

/// Buy fees enter cost basis: buy = 507 USD, proceeds = 493 USD,
/// realized result = 493 − 507 = −14 USD.
#[test]
fn fees_are_part_of_cost_and_reduce_proceeds() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(50), "USD").with_fees(dec!(7)),
        Transaction::sell(ACC, AAPL, d(2024, 2, 10), dec!(10), dec!(50), "USD").with_fees(dec!(7)),
    ];
    let holdings = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    assert_eq!(holdings.realized_pnl_base, dec!(-14));
    assert_eq!(holdings.positions[AAPL].quantity, Decimal::ZERO);
}

/// Selling more than held is an error, not a short position.
#[test]
fn overselling_is_rejected() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(5), dec!(100), "USD"),
        Transaction::sell(ACC, AAPL, d(2024, 2, 10), dec!(6), dec!(100), "USD"),
    ];
    assert!(build_holdings(&txs, "USD", &FakeRates::new()).is_err());
}

/// Missing FX is an error; silently guessing would be worse.
#[test]
fn missing_fx_rate_is_an_error() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 1, 10),
        dec!(5),
        dec!(100),
        "USD",
    )];
    let err = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap_err();
    assert!(matches!(err, sq_core::Error::MissingMarketData { .. }));
}

/// One security in two depots: A = 10 − 4 = 6, B = 5, portfolio quantity = 11;
/// return is per instrument and account quantities must sum to 11.
#[test]
fn a_security_held_in_two_depots_reports_both_accounts() {
    let rates = FakeRates::new();
    let txns = vec![
        Transaction::buy("depot-a", AAPL, d(2024, 1, 10), dec!(10), dec!(100), "EUR"),
        Transaction::buy("depot-b", AAPL, d(2024, 2, 10), dec!(5), dec!(110), "EUR"),
        Transaction::sell("depot-a", AAPL, d(2024, 3, 10), dec!(4), dec!(120), "EUR"),
    ];

    let holdings = build_holdings(&txns, "EUR", &rates).unwrap();
    let position = &holdings.positions[AAPL];

    assert_eq!(position.quantity, dec!(11));
    assert_eq!(position.accounts["depot-a"], dec!(6));
    assert_eq!(position.accounts["depot-b"], dec!(5));
    assert_eq!(
        position.accounts.values().sum::<Decimal>(),
        position.quantity,
        "the per-account sum must equal the position quantity"
    );
}

/// An account with no remaining quantity is omitted from the breakdown.
#[test]
fn an_emptied_depot_leaves_the_breakdown() {
    let rates = FakeRates::new();
    let txns = vec![
        Transaction::buy("depot-a", AAPL, d(2024, 1, 10), dec!(4), dec!(100), "EUR"),
        Transaction::buy("depot-b", AAPL, d(2024, 1, 11), dec!(6), dec!(100), "EUR"),
        Transaction::sell("depot-a", AAPL, d(2024, 5, 10), dec!(4), dec!(130), "EUR"),
    ];

    let holdings = build_holdings(&txns, "EUR", &rates).unwrap();
    let position = &holdings.positions[AAPL];

    assert_eq!(position.quantity, dec!(6));
    assert!(!position.accounts.contains_key("depot-a"));
    assert_eq!(position.accounts["depot-b"], dec!(6));
}

/// 10 shares bought at 100 USD when USD/EUR was 0.90: 1000 USD, 900 EUR.
/// At 120 USD and 0.95 the position is worth 1140 EUR, so 240 EUR unrealized.
/// The same 1000 USD is worth 1000 × 0.95 = 950 EUR today, so 50 EUR of that result is
/// the rate and 190 EUR is the instrument — which is also (1200 − 1000) × 0.95.
#[test]
fn an_unrealized_result_splits_into_instrument_and_currency() {
    let buy = Transaction::buy(ACC, AAPL, d(2024, 1, 2), dec!(10), dec!(100), "USD").with_fx_rate(dec!(0.90));

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 12, 31), dec!(120));
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 1, 2), dec!(0.90))
        .with("USD", "EUR", d(2024, 12, 31), dec!(0.95));

    let holdings = build_holdings(&[buy], "EUR", &rates).unwrap();
    let valuation = value_holdings(&holdings, "EUR", d(2024, 12, 31), &prices, &rates).unwrap();
    let position = &valuation.positions[0];

    assert_eq!(position.cost_fx_rate, dec!(0.95));
    assert_eq!(position.unrealized_pnl_base, dec!(240.00));
    assert_eq!(position.currency_gain_base, dec!(50.00));
    assert_eq!(position.instrument_gain_base(), dec!(190.00));
    assert_eq!(valuation.currency_gain_base, dec!(50.00));
}

/// The same split on a sale, fixed at the sale's own rate: 1000 USD of cost became
/// 900 EUR, the 1200 USD of proceeds convert at 0.95 to 1140 EUR.
/// Result 240 EUR = 50 EUR of currency + 190 EUR of instrument.
#[test]
fn a_realized_result_splits_the_same_way() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 2), dec!(10), dec!(100), "USD").with_fx_rate(dec!(0.90)),
        Transaction::sell(ACC, AAPL, d(2024, 12, 31), dec!(10), dec!(120), "USD").with_fx_rate(dec!(0.95)),
    ];

    let holdings = build_holdings(&transactions, "EUR", &FakeRates::new()).unwrap();
    let gain = &holdings.realized[0];

    assert_eq!(gain.cost_base, dec!(900.00));
    assert_eq!(gain.cost_in_currency, dec!(1000));
    assert_eq!(gain.gain_base, dec!(240.00));
    assert_eq!(gain.currency_gain_base, dec!(50.00));
    assert_eq!(gain.instrument_gain_base(), dec!(190.00));
    assert_eq!(holdings.realized_currency_gain_base, dec!(50.00));
}

/// The two cost-basis methods, side by side on one partly sold position.
///
/// Buy 10 @ 100 = 1000, buy 10 @ 150 = 1500, sell 10 @ 200 = 2000, price at the end 180.
///
/// FIFO sells the first ten shares:
///   realised   = 2000 − 1000 = +1000
///   left       = the 150 lot: cost 1500, 150 per share
///   unrealised = 10 × 180 − 1500 = 1800 − 1500 = +300
///
/// Moving average holds one lot at (1000 + 1500) / 20 = 125:
///   realised   = 2000 − 10 × 125 = 2000 − 1250 = +750
///   left       = cost 1250, 125 per share
///   unrealised = 1800 − 1250 = +550
///
/// Both add up to 1300: the methods move the result between realised and unrealised, they
/// never create or destroy it. The purchase values differ by 1500 − 1250 = 250.
#[test]
fn fifo_and_moving_average_split_one_position_differently() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 3, 10), dec!(10), dec!(150), "EUR"),
        Transaction::sell(ACC, AAPL, d(2024, 6, 10), dec!(10), dec!(200), "EUR"),
    ];
    let date = d(2024, 12, 31);
    let prices = FakePrices::new("EUR").with(AAPL, date, dec!(180));
    let rates = FakeRates::new();

    let pass = |method: CostBasisMethod| {
        build_holdings_with(
            &txs,
            "EUR",
            &rates,
            HoldingsOptions::default().with_cost_basis(method),
        )
        .unwrap()
    };
    let fifo = pass(CostBasisMethod::Fifo);
    let average = pass(CostBasisMethod::AverageCost);
    let valuation = value_holdings(&fifo, "EUR", date, &prices, &rates).unwrap();

    let rows = compare_cost_basis(&fifo, &average, &valuation);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];

    assert_eq!(
        row.quantity,
        dec!(10),
        "the quantity left owes nothing to the method"
    );
    assert_eq!(row.market_value_base, dec!(1800));

    assert_eq!(row.fifo.cost_basis_base, dec!(1500));
    assert_eq!(row.fifo.cost_per_unit_base, dec!(150));
    assert_eq!(row.fifo.unrealized_pnl_base, dec!(300));
    assert_eq!(row.fifo.realized_pnl_base, dec!(1000));

    assert_eq!(row.average.cost_basis_base, dec!(1250));
    assert_eq!(row.average.cost_per_unit_base, dec!(125));
    assert_eq!(row.average.unrealized_pnl_base, dec!(550));
    assert_eq!(row.average.realized_pnl_base, dec!(750));

    // Realised plus unrealised is the same total either way: 1000 + 300 = 750 + 550.
    assert_eq!(
        row.fifo.realized_pnl_base + row.fifo.unrealized_pnl_base,
        row.average.realized_pnl_base + row.average.unrealized_pnl_base
    );
    assert_eq!(row.spread_base(), dec!(250));
}

/// A position never partly sold cannot disagree with itself: one purchase, no disposal, so
/// both methods hold the same lot at the same price and the spread is zero.
#[test]
fn an_untouched_position_reads_the_same_under_both_methods() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 1, 10),
        dec!(4),
        dec!(250),
        "EUR",
    )];
    let date = d(2024, 6, 30);
    let prices = FakePrices::new("EUR").with(AAPL, date, dec!(300));
    let rates = FakeRates::new();

    let pass = |method: CostBasisMethod| {
        build_holdings_with(
            &txs,
            "EUR",
            &rates,
            HoldingsOptions::default().with_cost_basis(method),
        )
        .unwrap()
    };
    let fifo = pass(CostBasisMethod::Fifo);
    let average = pass(CostBasisMethod::AverageCost);
    let valuation = value_holdings(&fifo, "EUR", date, &prices, &rates).unwrap();

    let row = &compare_cost_basis(&fifo, &average, &valuation)[0];
    assert_eq!(row.fifo, row.average);
    assert_eq!(row.spread_base(), Decimal::ZERO);
    // 4 × 300 − 1000 = 200 either way.
    assert_eq!(row.fifo.unrealized_pnl_base, dec!(200));
}

/// The FIRE reading: a target, where the portfolio stands against it, and when the pace gets there.
///
/// Spending 24 000 a year at a 4% withdrawal rate needs 24 000 / 0.04 = 600 000.
/// The portfolio is worth 200 000, so it is 200 000 / 600 000 = 1/3 of the way and 400 000 short,
/// and what it holds today would sustain 200 000 × 0.04 = 8 000 a year.
///
/// Saving 1 500 a month at 5% a year, compounded monthly — i = 1.05^(1/12) − 1 ≈ 0.00407412 —
/// the balance after n months is 200000·(1+i)^n + 1500·((1+i)^n − 1)/i. Setting that to 600 000:
///   n = ln((600000·i + 1500) / (200000·i + 1500)) / ln(1+i) ≈ 131.09
/// Month 131 ends at 599 656, still short, so the answer is month 132 (603 599) — eleven years,
/// landing on 2037-01-31.
#[test]
fn fire_names_the_month_the_target_is_first_reached() {
    let assumptions = FireAssumptions {
        annual_spending: dec!(24000),
        withdrawal_rate: dec!(0.04),
        expected_return: dec!(0.05),
        monthly_contribution: dec!(1500),
    };
    let projection = fire_projection(dec!(200000), assumptions, d(2026, 1, 31)).unwrap();

    assert_eq!(projection.target_base, dec!(600000));
    assert_eq!(projection.missing_base, dec!(400000));
    assert_eq!(projection.sustainable_annual_base, dec!(8000));
    assert_eq!(projection.progress.round_dp(4), dec!(0.3333));
    assert_eq!(projection.months_to_target, Some(132));
    assert_eq!(projection.target_date, Some(d(2037, 1, 31)));
}

/// A portfolio already past its target is there today, not in some number of months, and its
/// progress reads above one rather than being capped at it.
#[test]
fn a_portfolio_past_its_target_is_already_there() {
    let projection = fire_projection(
        dec!(750000),
        FireAssumptions {
            annual_spending: dec!(24000),
            withdrawal_rate: dec!(0.04),
            expected_return: dec!(0.05),
            monthly_contribution: dec!(1500),
        },
        d(2026, 1, 31),
    )
    .unwrap();

    assert_eq!(projection.months_to_target, Some(0));
    assert_eq!(projection.target_date, Some(d(2026, 1, 31)));
    assert_eq!(projection.missing_base, Decimal::ZERO, "nothing is missing");
    assert_eq!(projection.progress, dec!(1.25)); // 750000 / 600000
}

/// Without growth the contributions alone close the gap: 400 000 missing at 2 000 a month is
/// exactly 200 months, and the month it lands on is 2042-09-30 — the last day of a short month,
/// because a date cannot move to a day its month does not have.
#[test]
fn without_growth_the_horizon_is_the_gap_over_the_contribution() {
    let projection = fire_projection(
        dec!(200000),
        FireAssumptions {
            annual_spending: dec!(24000),
            withdrawal_rate: dec!(0.04),
            expected_return: Decimal::ZERO,
            monthly_contribution: dec!(2000),
        },
        d(2026, 1, 31),
    )
    .unwrap();

    assert_eq!(projection.months_to_target, Some(200));
    assert_eq!(projection.target_date, Some(d(2042, 9, 30)));
}
