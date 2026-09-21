use super::*;

/// Dividends do not change quantity; 100 USD − 15 tax = 85 USD = 76.50 EUR at 0.90.
#[test]
fn dividend_net_of_tax_in_base_currency() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(50), "USD").with_fx_rate(dec!(0.90)),
        Transaction::dividend(ACC, AAPL, d(2024, 3, 1), dec!(100), "USD")
            .with_taxes(dec!(15))
            .with_fx_rate(dec!(0.90)),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    assert_eq!(holdings.dividends_base, dec!(76.500));
    assert_eq!(
        holdings.positions[AAPL].quantity,
        dec!(10),
        "a dividend does not change the quantity"
    );
    // Cash balance stays in transaction currency: −500 purchase, +85 dividend.
    assert_eq!(holdings.cash["USD"], dec!(-415));
}

/// Income is an event series: dividends = 125 EUR, interest = 6 EUR, total = 131 EUR.
#[test]
fn income_records_every_payment_including_the_nameless_ones() {
    let txs = vec![
        Transaction::dividend(ACC, AAPL, d(2024, 3, 15), dec!(100), "EUR").with_taxes(dec!(15)),
        Transaction::cash(ACC, TransactionKind::Dividend, d(2024, 6, 20), dec!(40), "EUR"),
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 6, 30), dec!(10), "EUR"),
        Transaction::cash(
            ACC,
            TransactionKind::InterestCharge,
            d(2024, 12, 31),
            dec!(4),
            "EUR",
        ),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();

    assert_eq!(holdings.income.len(), 4);
    assert_eq!(holdings.dividends_base, dec!(125));
    assert_eq!(holdings.interest_base, dec!(6));

    // Event-series totals must match the aggregate totals.
    let total = income_total(&holdings.income);
    assert_eq!(total.events, 4);
    assert_eq!(total.net_base, holdings.dividends_base + holdings.interest_base);
    assert_eq!(total.taxes_base, dec!(15));

    let by_kind = income_by_kind(&holdings.income);
    assert_eq!(by_kind[&TransactionKind::Dividend].net_base, dec!(125));
    assert_eq!(by_kind[&TransactionKind::Dividend].events, 2);
    assert_eq!(by_kind[&TransactionKind::Interest].net_base, dec!(10));
    // Expenses are negative so grouping can sum them directly.
    assert_eq!(by_kind[&TransactionKind::InterestCharge].net_base, dec!(-4));

    let by_month = income_by_month(&holdings.income);
    assert_eq!(by_month[&(2024, 3)].net_base, dec!(85));
    assert_eq!(by_month[&(2024, 6)].net_base, dec!(50)); // 40 dividend + 10 interest
    assert_eq!(by_month[&(2024, 12)].net_base, dec!(-4));

    // Only the named dividend belongs to the security breakdown.
    let by_security = income_by_security(&holdings.income);
    assert_eq!(by_security.len(), 1);
    assert_eq!(by_security[AAPL].net_base, dec!(85));

    // Dividend totals are unchanged, but now include the unnamed 40 EUR.
    assert_eq!(dividends_by_year(&holdings.income)[&2024].net_base, dec!(125));
    assert_eq!(dividends_by_year(&holdings.income)[&2024].payments, 2);
}

/// Calendar bottom row and year composition over two years:
/// June = 10 (2024 interest) + 30 (2025 dividend) = 40; 2025 = 30 dividend + 5 interest = 35.
/// Narrowed to dividends only, June = 30 and the 2024 total drops to 100 − 15 = 85.
#[test]
fn income_folds_years_into_months_and_splits_years_into_kinds() {
    let txs = vec![
        Transaction::dividend(ACC, AAPL, d(2024, 3, 15), dec!(100), "EUR").with_taxes(dec!(15)),
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 6, 30), dec!(10), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2025, 6, 18), dec!(30), "EUR"),
        Transaction::cash(ACC, TransactionKind::Interest, d(2025, 9, 30), dec!(5), "EUR"),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();

    // Same month across every year: March 85, June 10 + 30 = 40, September 5.
    let by_month_of_year = income_by_month_of_year(&holdings.income);
    assert_eq!(by_month_of_year[&3].net_base, dec!(85));
    assert_eq!(by_month_of_year[&6].net_base, dec!(40));
    assert_eq!(by_month_of_year[&6].events, 2);
    assert_eq!(by_month_of_year[&9].net_base, dec!(5));

    // Each year split by kind: 2024 = 85 dividend + 10 interest, 2025 = 30 + 5.
    let by_year_kind = income_by_year_kind(&holdings.income);
    assert_eq!(
        by_year_kind[&(2024, TransactionKind::Dividend)].net_base,
        dec!(85)
    );
    assert_eq!(
        by_year_kind[&(2024, TransactionKind::Interest)].net_base,
        dec!(10)
    );
    assert_eq!(
        by_year_kind[&(2025, TransactionKind::Dividend)].net_base,
        dec!(30)
    );
    assert_eq!(by_year_kind[&(2025, TransactionKind::Interest)].net_base, dec!(5));
    // The split adds back up to the plain year totals: 85 + 10 = 95, 30 + 5 = 35.
    let by_year = income_by_year(&holdings.income);
    assert_eq!(by_year[&2024].net_base, dec!(95));
    assert_eq!(by_year[&2025].net_base, dec!(35));

    // One kind only: interest disappears from every rollup, dividends keep their figures.
    let dividends = income_of_kind(&holdings.income, Some(TransactionKind::Dividend));
    assert_eq!(dividends.len(), 2);
    assert_eq!(income_total(&dividends).net_base, dec!(115)); // 85 + 30
    assert_eq!(income_by_month_of_year(&dividends)[&6].net_base, dec!(30));
    assert_eq!(income_by_year(&dividends)[&2024].net_base, dec!(85));
    assert!(income_by_year(&dividends).contains_key(&2025));
    assert!(!income_by_kind(&dividends).contains_key(&TransactionKind::Interest));

    // None keeps everything, so the unfiltered call is the same series.
    assert_eq!(
        income_of_kind(&holdings.income, None).len(),
        holdings.income.len()
    );
}

/// Foreign-currency dividend: gross = 27 EUR, tax = 4.05 EUR, net = 22.95 EUR;
/// `gross_in_currency` remains USD.
#[test]
fn income_keeps_the_amount_the_broker_printed() {
    let txs = vec![
        Transaction::dividend(ACC, AAPL, d(2024, 3, 15), dec!(30), "USD")
            .with_taxes(dec!(4.50))
            .with_fx_rate(dec!(0.90)),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    let record = &holdings.income[0];

    assert_eq!(record.gross_base, dec!(27.00));
    assert_eq!(record.taxes_base, dec!(4.050));
    assert_eq!(record.net_base, dec!(22.950));
    assert_eq!(record.currency, "USD");
    assert_eq!(record.gross_in_currency, dec!(30));
    assert_eq!(record.account_id, ACC);
}

/// Monthly journal uses each operation's FX: June = 1000 − 900 = 100 EUR;
/// July = 40 × 0.80 − 10 = 22 EUR.
#[test]
fn journal_totals_convert_each_month_at_its_own_rate() {
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 1), dec!(0.85))
        .with("USD", "EUR", d(2024, 7, 1), dec!(0.80));

    let txns = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD")
            .with_fx_from_base_total(dec!(900)),
        Transaction::cash(ACC, TransactionKind::Dividend, d(2024, 7, 10), dec!(40), "USD"),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 7, 20), dec!(10), "EUR"),
    ];

    let months = transactions_net_by_month(&txns, "EUR", &rates).unwrap();

    assert_eq!(months.len(), 2);
    assert_eq!((months[0].year, months[0].month), (2024, 6));
    assert_eq!(months[0].count, 2);
    assert_eq!(months[0].net_base.round_dp(2), dec!(100.00));

    assert_eq!((months[1].year, months[1].month), (2024, 7));
    assert_eq!(months[1].count, 2);
    assert_eq!(months[1].net_base.round_dp(2), dec!(22.00));

    // Monthly total equals the base-currency cash change for the period.
    let total: Decimal = months.iter().map(|m| m.net_base).sum();
    assert_eq!(total.round_dp(2), dec!(122.00));
}

/// Yearly journal folds months: 2023 = −500 EUR; 2024 = 100 + 22 = 122 EUR.
#[test]
fn journal_year_totals_fold_their_own_months() {
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 1), dec!(0.85))
        .with("USD", "EUR", d(2024, 7, 1), dec!(0.80));

    let txns = vec![
        Transaction::cash(ACC, TransactionKind::Withdrawal, d(2023, 12, 1), dec!(500), "EUR"),
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD")
            .with_fx_from_base_total(dec!(900)),
        Transaction::cash(ACC, TransactionKind::Dividend, d(2024, 7, 10), dec!(40), "USD"),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 7, 20), dec!(10), "EUR"),
    ];

    let months = transactions_net_by_month(&txns, "EUR", &rates).unwrap();
    let years = transactions_net_by_year(&months);

    assert_eq!(years.len(), 2);
    assert_eq!(years[0].year, 2023);
    assert_eq!(years[0].count, 1);
    assert_eq!(years[0].net_base.round_dp(2), dec!(-500.00));

    assert_eq!(years[1].year, 2024);
    assert_eq!(years[1].count, 4);
    assert_eq!(years[1].net_base.round_dp(2), dec!(122.00));

    // Year and month totals must agree.
    let by_year: Decimal = years.iter().map(|y| y.net_base).sum();
    let by_month: Decimal = months.iter().map(|m| m.net_base).sum();
    assert_eq!(by_year.round_dp(2), by_month.round_dp(2));
}

/// Core year-over-year income: 2023 = 85 EUR; 2024 = 109 EUR; growth = +24 EUR.
#[test]
fn income_by_year_totals_dividends_and_interest_together() {
    let rates = FakeRates::new();
    let mut d2023 = Transaction::cash(ACC, TransactionKind::Dividend, d(2023, 5, 2), dec!(100), "EUR");
    d2023.taxes = dec!(15);
    d2023.security_id = Some(AAPL.to_string());
    let mut d2024 = Transaction::cash(ACC, TransactionKind::Dividend, d(2024, 5, 2), dec!(120), "EUR");
    d2024.taxes = dec!(18);
    d2024.security_id = Some(AAPL.to_string());

    let txns = vec![
        d2023,
        d2024,
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 12, 31), dec!(7), "EUR"),
    ];

    let holdings = build_holdings(&txns, "EUR", &rates).unwrap();
    let by_year = income_by_year(&holdings.income);

    assert_eq!(by_year[&2023].net_base, dec!(85));
    assert_eq!(by_year[&2024].net_base, dec!(109));
    assert_eq!(by_year[&2024].events, 2);
    assert_eq!(by_year[&2024].net_base - by_year[&2023].net_base, dec!(24));

    // The annual result must match the monthly fold.
    let months: Decimal = income_by_month(&holdings.income)
        .iter()
        .filter(|((year, _), _)| *year == 2024)
        .map(|(_, s)| s.net_base)
        .sum();
    assert_eq!(months, dec!(109));
}

/// Four payments about 92 days apart read as quarterly, and the instrument's
/// schedule is built from its own payments only.
#[test]
fn a_payment_history_names_its_own_schedule() {
    let transactions = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 2), dec!(10), dec!(100), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 3, 15), dec!(10), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 6, 14), dec!(11), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 9, 14), dec!(12), "EUR"),
        Transaction::dividend(ACC, AAPL, d(2024, 12, 15), dec!(13), "EUR"),
    ];

    let holdings = build_holdings(&transactions, "EUR", &FakeRates::new()).unwrap();
    let profiles = dividend_profiles(&holdings.income, d(2025, 1, 31));
    let profile = &profiles[AAPL];

    assert_eq!(profile.payments, 4);
    assert_eq!(profile.frequency, DividendFrequency::Quarterly);
    assert_eq!(profile.median_gap_days, Some(92));
    assert_eq!(profile.last_payment, Some(d(2024, 12, 15)));
    assert_eq!(profile.net_base, dec!(46));
    assert_eq!(profile.trailing_year_base, dec!(46));
}

/// The payments grid lays one quarter of history on one axis, in USD so no rate is in the way.
///
/// Q1: deposit 1000, buy 10 x 50 = 500, dividend 30 − 5 tax = 25 net, standalone fee 2.
/// Q2: sell 10 x 60 = 600 (cost 500, so a realized +100), dividend 40 − 0 = 40, interest 3.
///
/// Rows come out as:
///   dividends    Q1 25,   Q2 40   → 65
///   interest     Q1 0,    Q2 3    → 3
///   fees         Q1 2,    Q2 0    → 2
///   savings      Q1 1000, Q2 0    → 1000   (the buy is internal, the deposit is not)
///   closed       Q1 0,    Q2 100  → 100
/// Earnings are income only: Q1 = 25, Q2 = 43, and the cumulative line reads 25 then 68.
#[test]
fn the_payments_grid_puts_every_line_on_one_axis() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 5), dec!(1000), "USD"),
        Transaction::buy(ACC, AAPL, d(2024, 2, 1), dec!(10), dec!(50), "USD"),
        Transaction::dividend(ACC, AAPL, d(2024, 3, 1), dec!(30), "USD").with_taxes(dec!(5)),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 3, 20), dec!(2), "USD"),
        Transaction::sell(ACC, AAPL, d(2024, 4, 15), dec!(10), dec!(60), "USD"),
        Transaction::dividend(ACC, AAPL, d(2024, 5, 10), dec!(40), "USD"),
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 6, 30), dec!(3), "USD"),
    ];
    let holdings = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    let grid = payment_grid(&holdings, d(2024, 1, 1), d(2024, 6, 30), PaymentPeriod::Quarter);

    assert_eq!(grid.buckets.len(), 2);
    assert_eq!(grid.buckets[1].from, d(2024, 4, 1));

    let row = |line: PaymentLine| {
        grid.lines
            .iter()
            .find(|row| row.line == line)
            .unwrap_or_else(|| panic!("{line:?} is missing"))
    };
    assert_eq!(row(PaymentLine::Dividends).amounts, vec![dec!(25), dec!(40)]);
    assert_eq!(row(PaymentLine::Interest).amounts, vec![Decimal::ZERO, dec!(3)]);
    assert_eq!(row(PaymentLine::Fees).amounts, vec![dec!(2), Decimal::ZERO]);
    assert_eq!(row(PaymentLine::Savings).amounts, vec![dec!(1000), Decimal::ZERO]);
    assert_eq!(
        row(PaymentLine::ClosedTrades).amounts,
        vec![Decimal::ZERO, dec!(100)]
    );

    // A fee is what the portfolio cost, a deposit is not a result: neither joins the subtotal.
    assert_eq!(grid.earnings, vec![dec!(25), dec!(43)]);
    assert_eq!(grid.cumulative, vec![dec!(25), dec!(68)]);
    assert_eq!(grid.earnings_total, dec!(68));

    // Two payers, largest first: the instrument paid 25 + 40 = 65, the account itself 3.
    assert_eq!(grid.securities.len(), 2);
    assert_eq!(grid.securities[0].security_id.as_deref(), Some(AAPL));
    assert_eq!(grid.securities[0].total, dec!(65));
    assert_eq!(
        grid.securities[1].security_id, None,
        "account interest has no payer"
    );
    assert_eq!(grid.securities[1].amounts, vec![Decimal::ZERO, dec!(3)]);
}

/// A classification tree splits income the way it splits value: the payer's weights.
///
/// Tree: Equity → {US, Europe}, and Cash beside them. AAPL is 60% US / 40% Europe, the
/// account's EUR balance is all Cash, the bond fund is classified nowhere.
///
/// Payments (EUR, so no rate is in the way):
///   AAPL dividend  100 gross − 15 tax = 85 net
///   BOND dividend   50 gross          = 50 net
///   interest                            10 net
///   interest charge                     −4 net
///
/// Longhand:
///   US        = 85 × 0.6 = 51,  tax 15 × 0.6 = 9
///   Europe    = 85 × 0.4 = 34,  tax 15 × 0.4 = 6
///   Equity    = 51 + 34  = 85,  one payment reached it, not two
///   Cash      = 10 − 4   = 6
///   unclassified = 50
///   total     = 85 + 6 + 50 = 141, and 85/141 is Equity's share
#[test]
fn a_taxonomy_splits_income_the_way_it_splits_value() {
    const BOND: &str = "sec-bond";
    let equity = TaxonomyNode::root("tax-1", "Equity");
    let us = TaxonomyNode::child(&equity, "US");
    let europe = TaxonomyNode::child(&equity, "Europe");
    let cash_node = TaxonomyNode::root("tax-1", "Cash");
    let nodes = vec![equity.clone(), us.clone(), europe.clone(), cash_node.clone()];

    let cash_key = cash_subject_key(ACC, "EUR");
    let assignments = vec![
        Assignment {
            subject_id: AAPL.to_string(),
            node_id: us.id.clone(),
            weight: dec!(0.6),
        },
        Assignment {
            subject_id: AAPL.to_string(),
            node_id: europe.id.clone(),
            weight: dec!(0.4),
        },
        Assignment {
            subject_id: cash_key.clone(),
            node_id: cash_node.id.clone(),
            weight: dec!(1),
        },
    ];

    let txs = vec![
        Transaction::dividend(ACC, AAPL, d(2024, 3, 15), dec!(100), "EUR").with_taxes(dec!(15)),
        Transaction::dividend(ACC, BOND, d(2024, 5, 2), dec!(50), "EUR"),
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 6, 30), dec!(10), "EUR"),
        Transaction::cash(
            ACC,
            TransactionKind::InterestCharge,
            d(2024, 12, 31),
            dec!(4),
            "EUR",
        ),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    let split = income_by_taxonomy(&holdings.income, &nodes, &assignments, &[]);

    assert_eq!(split.total.net_base, dec!(141));
    assert_eq!(split.total.events, 4);

    let node = |key: &str| {
        fn walk<'a>(nodes: &'a [IncomeNode], key: &str) -> Option<&'a IncomeNode> {
            nodes
                .iter()
                .find(|n| n.key == key)
                .or_else(|| nodes.iter().find_map(|n| walk(&n.children, key)))
        }
        walk(&split.nodes, key).unwrap_or_else(|| panic!("{key} is missing"))
    };

    assert_eq!(node(&us.id).summary.net_base, dec!(51.0));
    assert_eq!(node(&us.id).summary.taxes_base, dec!(9.0));
    assert_eq!(node(&europe.id).summary.net_base, dec!(34.0));
    assert_eq!(node(&europe.id).summary.taxes_base, dec!(6.0));
    assert_eq!(node(&equity.id).summary.net_base, dec!(85.0));
    assert_eq!(
        node(&equity.id).summary.events,
        1,
        "one dividend split in two is still one payment"
    );
    assert_eq!(node(&cash_node.id).summary.net_base, dec!(6));
    assert_eq!(node(&cash_node.id).summary.events, 2);

    // The bond fund is in no node at all, so it lands in the remainder, not in a zero bucket.
    let rest = node(UNCLASSIFIED_KEY);
    assert_eq!(rest.summary.net_base, dec!(50));
    assert_eq!(rest.summary.events, 1);

    // The tree adds back up to the total it was given: 85 + 6 + 50 = 141.
    let sum: Decimal = split.nodes.iter().map(|n| n.summary.net_base).sum();
    assert_eq!(sum, split.total.net_base);
    assert_eq!(node(&equity.id).weight, dec!(85) / dec!(141));

    // Excluding the bond fund takes its payment out of the tree and out of the denominator —
    // it is not moved to the remainder: 141 − 50 = 91.
    let without = income_by_taxonomy(&holdings.income, &nodes, &assignments, &[BOND.to_string()]);
    assert_eq!(without.total.net_base, dec!(91));
    assert_eq!(without.total.events, 3);
    assert!(without.nodes.iter().all(|n| n.key != UNCLASSIFIED_KEY));
}
