//! Series, risk metrics, allocation, rebalancing, and reports (phase 2).

mod support;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::{
    Assignment, CASH_KEY, CashSubject, HoldingsOptions, MemberScope, RebalanceOptions, SubjectKind,
    UNCLASSIFIED_KEY, allocation_by_account, allocation_by_security, allocation_by_taxonomy,
    allocation_members, allocation_tree, benchmark_return, benchmark_series, benchmark_start, build_holdings,
    capital_gains_by_year, compare_to_benchmark, dividends_by_year, position_twr_between, position_xirr,
    rebalance, risk_report, taxonomy_subjects, twr_between, valuation_at, value_series, yield_on_cost,
};
use sq_core::market::DateRange;
use sq_core::model::{
    Account, AllocationTarget, CashClassification, Security, SecurityClassification, SecurityKind, Taxonomy,
    TaxonomyKind, TaxonomyNode, Transaction, TransactionKind,
};
use sq_core::storage::Store;
use support::{FakePrices, FakeRates, d};

const ACC: &str = "acc-1";
const ACC2: &str = "acc-2";
/// Cash accounts used by depots `ACC` and `ACC2`.
const CASH: &str = "cash-1";
const CASH2: &str = "cash-2";
const A: &str = "sec-a";
const B: &str = "sec-b";

/// EUR portfolio: 1000 deposited and 10 shares bought at 100 on June 3.
/// Quotes exist on June 3, 5, and 7; other days use forward-fill.
fn week_scenario() -> (Vec<Transaction>, FakePrices, FakeRates) {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(1000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(10), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 6, 3), dec!(100))
        .with(A, d(2024, 6, 5), dec!(110))
        .with(A, d(2024, 6, 7), dec!(90));
    (transactions, prices, FakeRates::new())
}

/// The series includes every calendar day and forward-fills missing quotes:
/// June 3 = 1000; June 5 = 1100; June 7–9 = 900.
#[test]
fn value_series_covers_every_day_and_fills_gaps() {
    let (transactions, prices, rates) = week_scenario();
    let series = value_series(
        &transactions,
        "EUR",
        DateRange::new(d(2024, 6, 3), d(2024, 6, 9)),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();

    assert_eq!(series.len(), 7);
    assert_eq!(series.total_value_base[0], dec!(1000));
    assert_eq!(series.total_value_base[1], dec!(1000));
    assert_eq!(series.total_value_base[2], dec!(1100));
    assert_eq!(series.total_value_base[3], dec!(1100));
    assert_eq!(series.total_value_base[4], dec!(900));
    assert_eq!(series.total_value_base[6], dec!(900));

    // The deposit appears only on its transaction date.
    assert_eq!(series.external_flow_base[0], dec!(1000));
    assert!(series.external_flow_base[1..].iter().all(|f| f.is_zero()));

    // Weekends are excluded from trading-day metrics.
    assert_eq!(series.business_days().len(), 5);
}

/// TWR from the series matches transactions: 1000 → 900 gives 900/1000 − 1 = −0.1.
#[test]
fn series_twr_matches_twr_from_transactions() {
    let (transactions, prices, rates) = week_scenario();
    let range = DateRange::new(d(2024, 6, 3), d(2024, 6, 9));

    let from_series = value_series(
        &transactions,
        "EUR",
        range,
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap()
    .twr()
    .unwrap();
    let from_transactions = twr_between(&transactions, "EUR", range.from, range.to, &prices, &rates).unwrap();

    assert_eq!(from_series.round_dp(10), dec!(-0.1));
    assert_eq!(from_series.round_dp(10), from_transactions.round_dp(10));
}

/// Portfolio total 10,000: stocks = 7000 (70%), cash = 2000 (20%),
/// unclassified = 1000 (10%); excluding B leaves 8000: stocks 75%, cash 25%.
#[test]
fn taxonomy_allocation_divides_securities_and_cash_and_skips_the_disabled() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(10000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(60), dec!(100), "EUR"),
        Transaction::buy(ACC, B, d(2024, 6, 3), dec!(20), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 6, 3), dec!(100))
        .with(B, d(2024, 6, 3), dec!(100));
    let rates = FakeRates::new();

    let valuation = valuation_at(&transactions, "EUR", d(2024, 6, 3), &prices, &rates).unwrap();

    let stocks = node_named("node-stocks", "Equities");
    let cash_node = node_named("node-cash", "Cash");
    let nodes = [stocks.clone(), cash_node.clone()];
    let securities = vec![
        Security::new(A, "Security A", "EUR", SecurityKind::Etf),
        Security::new(B, "Security B", "EUR", SecurityKind::Etf),
    ];
    let cash = vec![CashSubject {
        account_id: CASH.into(),
        account_name: "Cash account".into(),
        currency: "EUR".into(),
        value_base: dec!(2000),
    }];
    let assignments = vec![
        Assignment::from(&SecurityClassification::new(A, &stocks.id, Decimal::ONE)),
        Assignment::from(&SecurityClassification::new(B, &stocks.id, dec!(0.5))),
        Assignment::from(&CashClassification::new(CASH, "EUR", &cash_node.id, Decimal::ONE)),
    ];

    let subjects = taxonomy_subjects(&valuation, &securities, &cash, &[]);
    let allocation = allocation_by_taxonomy(&subjects, &nodes, &assignments);

    assert_eq!(allocation.total_base, dec!(10000));
    assert_eq!(allocation.find("node-stocks").unwrap().value_base, dec!(7000));
    assert_eq!(allocation.find("node-stocks").unwrap().weight, dec!(0.7));
    assert_eq!(allocation.find("node-cash").unwrap().value_base, dec!(2000));
    assert_eq!(allocation.find("node-cash").unwrap().weight, dec!(0.2));
    assert_eq!(allocation.find(UNCLASSIFIED_KEY).unwrap().weight, dec!(0.1));
    // This taxonomy has no generic cash bucket; cash is an explicit category.
    assert!(allocation.find(CASH_KEY).is_none());
    assert_eq!(allocation.total_weight(), Decimal::ONE);

    // Excluded securities leave both the denominator and unclassified bucket.
    let subjects = taxonomy_subjects(&valuation, &securities, &cash, &[B.to_string()]);
    let allocation = allocation_by_taxonomy(&subjects, &nodes, &assignments);

    assert_eq!(allocation.total_base, dec!(8000));
    assert_eq!(allocation.find("node-stocks").unwrap().weight, dec!(0.75));
    assert_eq!(allocation.find("node-cash").unwrap().weight, dec!(0.25));
    assert!(allocation.find(UNCLASSIFIED_KEY).is_none());
    assert_eq!(allocation.total_weight(), Decimal::ONE);
}

/// The same portfolio as one tree: A = 6000, B = 1000 under Stocks,
/// cash = 2000, and unclassified B = 1000; nested depth is preserved.
#[test]
fn taxonomy_tree_puts_subjects_under_their_nodes() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(10000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(60), dec!(100), "EUR"),
        Transaction::buy(ACC, B, d(2024, 6, 3), dec!(20), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 6, 3), dec!(100))
        .with(B, d(2024, 6, 3), dec!(100));
    let rates = FakeRates::new();
    let valuation = valuation_at(&transactions, "EUR", d(2024, 6, 3), &prices, &rates).unwrap();

    let stocks = node_named("node-stocks", "Equities");
    let mut core_node = node_named("node-core", "Core");
    core_node.parent_id = Some(stocks.id.clone());
    let cash_node = node_named("node-cash", "Cash");
    let nodes = [stocks.clone(), core_node.clone(), cash_node.clone()];
    // Explicit IDs let the tile label resolve the security by `security_id`.
    let mut sec_a = Security::new(
        "IWDA.L",
        "iShares Core MSCI World UCITS ETF",
        "EUR",
        SecurityKind::Etf,
    );
    sec_a.id = A.into();
    let mut sec_b = Security::new(
        "EIMI.L",
        "iShares Core MSCI EM IMI UCITS ETF",
        "EUR",
        SecurityKind::Etf,
    );
    sec_b.id = B.into();
    let securities = vec![sec_a, sec_b];
    let cash = vec![CashSubject {
        account_id: CASH.into(),
        account_name: "Cash account".into(),
        currency: "EUR".into(),
        value_base: dec!(2000),
    }];
    let assignments = vec![
        Assignment::from(&SecurityClassification::new(A, &core_node.id, Decimal::ONE)),
        Assignment::from(&SecurityClassification::new(B, &stocks.id, dec!(0.5))),
        Assignment::from(&CashClassification::new(CASH, "EUR", &cash_node.id, Decimal::ONE)),
    ];

    let subjects = taxonomy_subjects(&valuation, &securities, &cash, &[]);
    let tree = allocation_tree(&subjects, &nodes, &assignments);

    assert_eq!(tree.total_base, dec!(10000));
    assert_eq!(tree.total_weight(), Decimal::ONE);
    assert_eq!(tree.find("node-stocks").unwrap().value_base, dec!(7000));
    assert_eq!(tree.find("node-stocks").unwrap().weight, dec!(0.7));
    // Security A is a leaf under Core, so the map preserves nesting.
    let core_bucket = tree.find("node-core").unwrap();
    assert_eq!(core_bucket.value_base, dec!(6000));
    assert_eq!(core_bucket.children.len(), 1);
    assert_eq!(core_bucket.children[0].key, A);
    assert_eq!(core_bucket.children[0].weight, dec!(0.6));
    // B is split between Stocks and residual; both tiles share one security key.
    let stocks_bucket = tree.find("node-stocks").unwrap();
    let b_tile = stocks_bucket.children.iter().find(|c| c.key == B).unwrap();
    assert_eq!(b_tile.value_base, dec!(1000));
    let rest = tree.find(UNCLASSIFIED_KEY).unwrap();
    assert_eq!(rest.children.len(), 1);
    assert_eq!(rest.children[0].key, B);
    assert_eq!(rest.children[0].value_base, dec!(1000));
    // Security tiles use the ticker label.
    assert_eq!(core_bucket.children[0].label, "IWDA.L");
    // Cash tiles use the account name because currency alone is not unique.
    let cash_tile = &tree.find("node-cash").unwrap().children[0];
    assert_eq!(cash_tile.label, "Cash account");
    assert_eq!(cash_tile.value_base, dec!(2000));
    assert_eq!(cash_tile.weight, dec!(0.2));

    // Excluded securities disappear from the map and denominator.
    let subjects = taxonomy_subjects(&valuation, &securities, &cash, &[B.to_string()]);
    let tree = allocation_tree(&subjects, &nodes, &assignments);

    assert_eq!(tree.total_base, dec!(8000));
    assert_eq!(tree.total_weight(), Decimal::ONE);
    assert!(tree.find(UNCLASSIFIED_KEY).is_none());
    assert_eq!(tree.find("node-core").unwrap().children[0].weight, dec!(0.75));
}

/// Stocks composition: A = 6000, B = 2000 × 0.5 = 1000, total = 7000.
/// Weights are 0.857142857… and 0.142857142…; residual B = 1000.
#[test]
fn allocation_members_split_a_position_between_node_and_remainder() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(10000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(60), dec!(100), "EUR"),
        Transaction::buy(ACC, B, d(2024, 6, 3), dec!(20), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 6, 3), dec!(100))
        .with(B, d(2024, 6, 3), dec!(100));
    let rates = FakeRates::new();
    let valuation = valuation_at(&transactions, "EUR", d(2024, 6, 3), &prices, &rates).unwrap();

    let stocks = TaxonomyNode {
        id: "node-stocks".into(),
        taxonomy_id: "tax".into(),
        parent_id: None,
        name: "Equities".into(),
        rank: 0,
        color: None,
    };
    let assignments = vec![
        Assignment::from(&SecurityClassification::new(A, &stocks.id, Decimal::ONE)),
        Assignment::from(&SecurityClassification::new(B, &stocks.id, dec!(0.5))),
    ];
    let nodes = [stocks];
    let securities = vec![
        Security::new(A, "Security A", "EUR", SecurityKind::Etf),
        Security::new(B, "Security B", "EUR", SecurityKind::Etf),
    ];
    let subjects = taxonomy_subjects(&valuation, &securities, &[], &[]);

    let members = allocation_members(&subjects, &nodes, &assignments, MemberScope::Node("node-stocks"));

    assert_eq!(members.len(), 2);
    // Order is descending by allocation value.
    assert_eq!(members[0].subject_id, A);
    assert_eq!(members[0].kind, SubjectKind::Security);
    assert_eq!(members[0].value_base, dec!(6000));
    assert_eq!(members[0].subject_value_base, dec!(6000));
    assert_eq!(members[0].assigned_share, Decimal::ONE);
    assert_eq!(members[1].subject_id, B);
    assert_eq!(members[1].value_base, dec!(1000));
    assert_eq!(members[1].subject_value_base, dec!(2000));
    assert_eq!(members[1].assigned_share, dec!(0.5));
    // Node weights sum to 1, like the allocation view.
    assert_eq!(
        (members[0].weight + members[1].weight).round_dp(10),
        Decimal::ONE.round_dp(10)
    );
    // Composition value matches the allocation bucket.
    let sum: Decimal = members.iter().map(|m| m.value_base).sum();
    assert_eq!(sum, dec!(7000));

    let rest = allocation_members(&subjects, &nodes, &assignments, MemberScope::Unclassified);
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].subject_id, B);
    assert_eq!(rest[0].value_base, dec!(1000));
    assert_eq!(rest[0].assigned_share, dec!(0.5));

    // At the top level each position contributes its full value.
    let all = allocation_members(&subjects, &nodes, &assignments, MemberScope::Portfolio);
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|m| m.assigned_share == Decimal::ONE));
    assert_eq!(all[0].value_base, dec!(6000));

    // Excluded security remains listed with zero weight and a flag.
    let off = taxonomy_subjects(&valuation, &securities, &[], &[A.to_string()]);
    let all = allocation_members(&off, &nodes, &assignments, MemberScope::Portfolio);
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].subject_id, A, "an excluded subject sorts last");
    assert!(all[1].excluded);
    assert_eq!(all[1].weight, Decimal::ZERO);
    assert_eq!(
        all[0].weight,
        Decimal::ONE,
        "only included subjects divide the level"
    );
    // Excluded securities never enter unclassified.
    let rest = allocation_members(&off, &nodes, &assignments, MemberScope::Unclassified);
    assert!(rest.iter().all(|m| m.subject_id != A));
}

/// Node with a fixed ID, matching the bucket key used by the test.
fn node_named(id: &str, name: &str) -> TaxonomyNode {
    TaxonomyNode {
        id: id.into(),
        taxonomy_id: "tax".into(),
        parent_id: None,
        name: name.into(),
        rank: 0,
        color: None,
    }
}

/// Account allocation keeps securities on depots and cash on cash accounts:
/// pair values 3000/2000 and 1000/3000; total = 9000.
#[test]
fn allocation_by_account_settles_cash_on_the_deposit_account() {
    let transactions = vec![
        Transaction::cash(CASH, TransactionKind::Deposit, d(2024, 6, 3), dec!(5000), "EUR"),
        Transaction::cash(CASH2, TransactionKind::Deposit, d(2024, 6, 3), dec!(4000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(30), dec!(100), "EUR"),
        Transaction::buy(ACC2, A, d(2024, 6, 3), dec!(10), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR").with(A, d(2024, 6, 3), dec!(100));
    let rates = FakeRates::new();

    let mut first_cash = Account::deposit("Bank 1", "EUR");
    first_cash.id = CASH.into();
    let mut second_cash = Account::deposit("Bank 2", "EUR");
    second_cash.id = CASH2.into();
    let mut first = Account::securities("Broker 1", "EUR", CASH);
    first.id = ACC.into();
    let mut second = Account::securities("Broker 2", "EUR", CASH2);
    second.id = ACC2.into();
    let mut security = Security::new("A", "Security A", "EUR", SecurityKind::Etf);
    security.id = A.into();

    let allocation = allocation_by_account(
        &transactions,
        &[first_cash, second_cash, first, second],
        &[security],
        "EUR",
        d(2024, 6, 3),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();

    assert_eq!(allocation.total_base, dec!(9000));
    assert_eq!(allocation.find(ACC).unwrap().value_base, dec!(3000));
    assert_eq!(allocation.find(CASH).unwrap().value_base, dec!(2000));
    assert_eq!(allocation.find(ACC2).unwrap().value_base, dec!(1000));
    assert_eq!(allocation.find(CASH2).unwrap().value_base, dec!(3000));
}

/// Benchmark in USD, reporting in EUR: 360 → 418 EUR, return = 0.161111…;
/// versus −10% portfolio, excess = −0.261111.
#[test]
fn benchmark_return_is_measured_in_base_currency() {
    let prices =
        FakePrices::new("USD")
            .with("spy", d(2024, 6, 3), dec!(400))
            .with("spy", d(2024, 6, 7), dec!(440));
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 3), dec!(0.90))
        .with("USD", "EUR", d(2024, 6, 7), dec!(0.95));

    let benchmark = benchmark_return("spy", "EUR", d(2024, 6, 3), d(2024, 6, 7), &prices, &rates).unwrap();
    assert_eq!(benchmark.round_dp(6), dec!(0.161111));

    let comparison = compare_to_benchmark(d(2024, 6, 3), d(2024, 6, 7), dec!(-0.1), benchmark);
    assert_eq!(comparison.excess.round_dp(6), dec!(-0.261111));
}

/// Benchmark curve uses portfolio dates and ends at the benchmark return:
/// values 1, 1, 1.055555…, 1.161111… = 1 + benchmark_return.
#[test]
fn benchmark_series_shares_the_dates_of_the_portfolio() {
    let prices =
        FakePrices::new("USD")
            .with("spy", d(2024, 6, 3), dec!(400))
            .with("spy", d(2024, 6, 7), dec!(440));
    let rates = FakeRates::new()
        .with("USD", "EUR", d(2024, 6, 3), dec!(0.90))
        .with("USD", "EUR", d(2024, 6, 5), dec!(0.95));

    let dates = vec![d(2024, 6, 3), d(2024, 6, 4), d(2024, 6, 5), d(2024, 6, 7)];
    let growth = benchmark_series("spy", "EUR", &dates, &prices, &rates).unwrap();

    assert_eq!(growth.dates, dates);
    assert_eq!(growth.values[0], Decimal::ONE);
    assert_eq!(growth.values[1], Decimal::ONE);
    assert_eq!(growth.values[2].round_dp(6), dec!(1.055556));

    let total = benchmark_return("spy", "EUR", d(2024, 6, 3), d(2024, 6, 7), &prices, &rates).unwrap();
    assert_eq!(growth.values[3].round_dp(6), (Decimal::ONE + total).round_dp(6));
}

/// A benchmark listed halfway through the period: prices 100 on 06-05 and 110 on 06-07, none
/// before. The curve starts on 06-05 based at that price — 100/100 = 1, forward-filled 06-06 =
/// 100/100 = 1, 06-07 = 110/100 = 1.1 — and covers three of the portfolio's five days.
#[test]
fn benchmark_series_starts_where_the_benchmark_does() {
    let prices =
        FakePrices::new("USD")
            .with("new", d(2024, 6, 5), dec!(100))
            .with("new", d(2024, 6, 7), dec!(110));
    let rates = FakeRates::new();

    let dates = vec![
        d(2024, 6, 3),
        d(2024, 6, 4),
        d(2024, 6, 5),
        d(2024, 6, 6),
        d(2024, 6, 7),
    ];
    let growth = benchmark_series("new", "USD", &dates, &prices, &rates).unwrap();

    assert_eq!(growth.dates, dates[2..].to_vec());
    assert_eq!(growth.values, vec![dec!(1), dec!(1), dec!(1.1)]);
    assert_eq!(
        benchmark_start("new", &dates, &prices).unwrap(),
        Some(d(2024, 6, 5))
    );
}

/// A benchmark with no price anywhere in the window is a gap, not a young instrument.
#[test]
fn benchmark_series_without_any_price_is_missing_data() {
    let prices = FakePrices::new("USD").with("spy", d(2024, 7, 1), dec!(100));
    let dates = vec![d(2024, 6, 3), d(2024, 6, 4)];

    assert_eq!(benchmark_start("spy", &dates, &prices).unwrap(), None);
    assert!(matches!(
        benchmark_series("spy", "USD", &dates, &prices, &FakeRates::new()),
        Err(sq_core::error::Error::MissingMarketData { .. })
    ));
}

/// Portfolio curve uses the same returns as TWR, so its final point is `1 + TWR`.
#[test]
fn portfolio_growth_ends_at_the_declared_twr() {
    let (transactions, prices, rates) = week_scenario();
    let series = value_series(
        &transactions,
        "EUR",
        DateRange::new(d(2024, 6, 3), d(2024, 6, 7)),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();

    let growth = series.growth();
    assert_eq!(growth.dates.first().copied(), Some(d(2024, 6, 3)));
    assert_eq!(growth.values.first().copied(), Some(Decimal::ONE));
    assert_eq!(
        growth.values.last().copied().unwrap(),
        Decimal::ONE + series.twr().unwrap()
    );
}

/// The risk report shares the daily returns used by individual metrics.
#[test]
fn risk_report_agrees_with_the_metrics_it_reports() {
    let (transactions, prices, rates) = week_scenario();
    let series = value_series(
        &transactions,
        "EUR",
        DateRange::new(d(2024, 6, 3), d(2024, 6, 10)),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();

    let report = risk_report(&series, 0.0, 2);
    // June 3 has no previous day; the trading-day filter removes June 8–9.
    assert_eq!(report.returns.len(), report.metrics.days);
    assert_eq!(report.drawdown.len(), report.metrics.days);
    assert_eq!(report.window_days, 2);
    assert_eq!(report.episodes.first().cloned(), report.metrics.max_drawdown);
    // A rolling window of width 2 yields one fewer point than days.
    assert_eq!(report.rolling_volatility.len(), report.metrics.days - 1);
}

/// Income reports: buy cost = 1005, sale proceeds = 475, sold cost = 402,
/// realized result = 73; dividend net = 42.5; remainder value = 603.
#[test]
fn capital_gains_and_dividends_by_year() {
    let transactions = vec![
        Transaction::buy(ACC, A, d(2023, 3, 1), dec!(10), dec!(100), "EUR").with_fees(dec!(5)),
        Transaction::sell(ACC, A, d(2024, 5, 1), dec!(4), dec!(120), "EUR")
            .with_fees(dec!(3))
            .with_taxes(dec!(2)),
        Transaction::dividend(ACC, A, d(2024, 6, 1), dec!(50), "EUR").with_taxes(dec!(7.5)),
    ];
    let holdings = build_holdings(&transactions, "EUR", &FakeRates::new()).unwrap();

    let gains = capital_gains_by_year(&holdings.realized);
    let year = &gains[&2024];
    assert_eq!(year.disposals, 1);
    assert_eq!(year.proceeds_base, dec!(480));
    assert_eq!(year.fees_base, dec!(3));
    assert_eq!(year.taxes_base, dec!(2));
    assert_eq!(year.cost_base, dec!(402));
    assert_eq!(year.gain_base, dec!(73));
    // 2023 had no disposal, so it has no realized-income row.
    assert!(!gains.contains_key(&2023));

    let dividends = dividends_by_year(&holdings.income);
    assert_eq!(dividends[&2024].gross_base, dec!(50));
    assert_eq!(dividends[&2024].taxes_base, dec!(7.5));
    assert_eq!(dividends[&2024].net_base, dec!(42.5));

    let yoc = yield_on_cost(&holdings, A).unwrap();
    assert_eq!(yoc.round_dp(6), dec!(0.070481));
}

/// A security's return uses its own cash flows: 1000 → 1100,
/// TWR = 1100/1000 − 1 = 0.1; XIRR over 365 days = 0.1.
#[test]
fn position_twr_and_xirr_on_a_single_security() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 1, 1), dec!(1000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 1, 1), dec!(10), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(A, d(2024, 1, 1), dec!(100))
        .with(A, d(2024, 12, 31), dec!(110));
    let rates = FakeRates::new();

    let twr = position_twr_between(
        &transactions,
        A,
        "EUR",
        d(2024, 1, 1),
        d(2024, 12, 31),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();
    assert_eq!(twr.round_dp(10), dec!(0.1));

    let irr = position_xirr(
        &transactions,
        A,
        "EUR",
        d(2024, 12, 31),
        &prices,
        &rates,
        HoldingsOptions::default(),
    )
    .unwrap();
    assert_eq!(irr.round_dp(4), dec!(0.1));
}

/// Security allocation exposes cash as a separate bucket.
#[test]
fn allocation_by_security_lists_cash_separately() {
    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(1000), "EUR"),
        Transaction::buy(ACC, A, d(2024, 6, 3), dec!(6), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR").with(A, d(2024, 6, 3), dec!(100));
    let rates = FakeRates::new();
    let valuation = valuation_at(&transactions, "EUR", d(2024, 6, 3), &prices, &rates).unwrap();

    let mut security = Security::new("A", "Security A", "EUR", SecurityKind::Etf);
    security.id = A.into();
    let allocation = allocation_by_security(&valuation, &[security]);

    assert_eq!(allocation.find(A).unwrap().weight, dec!(0.6));
    assert_eq!(allocation.find(CASH_KEY).unwrap().weight, dec!(0.4));
    assert_eq!(allocation.total_weight(), Decimal::ONE);
}

/// Targets persist with their weights and round-trip unchanged.
#[test]
fn target_round_trips_through_storage() {
    let store = Store::open_in_memory().unwrap();
    let account = Account::deposit("Broker", "EUR");
    store.save_account(&account).unwrap();
    let portfolio = sq_core::model::Portfolio::new("Main", "EUR").with_accounts([account.id.clone()]);
    store.save_portfolio(&portfolio).unwrap();

    let taxonomy = Taxonomy::new("Regions", TaxonomyKind::Region);
    store.save_taxonomy(&taxonomy).unwrap();
    let us = TaxonomyNode::root(&taxonomy.id, "United States");
    let eu = TaxonomyNode::root(&taxonomy.id, "Europe");
    store.save_taxonomy_node(&us).unwrap();
    store.save_taxonomy_node(&eu).unwrap();

    let target = AllocationTarget::new(&portfolio.id, &taxonomy.id, "60/40")
        .with_weight(&us.id, dec!(0.6))
        .with_weight(&eu.id, dec!(0.4));
    store.save_target(&target).unwrap();

    let loaded = store.get_target(&target.id).unwrap();
    assert_eq!(loaded.total_weight(), Decimal::ONE);
    assert_eq!(loaded.weights.len(), 2);
    assert_eq!(store.targets_for_portfolio(&portfolio.id).unwrap().len(), 1);

    // A sum above one is invalid data, not something to normalize.
    let broken = AllocationTarget::new(&portfolio.id, &taxonomy.id, "broken")
        .with_weight(&us.id, dec!(0.8))
        .with_weight(&eu.id, dec!(0.4));
    assert!(store.save_target(&broken).is_err());
}

/// Nested target: stocks 60%, core 80%, defensive 20%; portfolio = core 5000,
/// defensive 1000, bonds 4000; drift −200/+200/0; core weight = 83.33% vs 80%.
#[test]
fn nested_target_multiplies_weights_and_leaves_trades_to_the_deepest_nodes() {
    const CORE: &str = "sec-core";
    const DEF: &str = "sec-def";
    const BOND: &str = "sec-bond";

    let transactions = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 3), dec!(10000), "EUR"),
        Transaction::buy(ACC, CORE, d(2024, 6, 3), dec!(50), dec!(100), "EUR"),
        Transaction::buy(ACC, DEF, d(2024, 6, 3), dec!(10), dec!(100), "EUR"),
        Transaction::buy(ACC, BOND, d(2024, 6, 3), dec!(40), dec!(100), "EUR"),
    ];
    let prices = FakePrices::new("EUR")
        .with(CORE, d(2024, 6, 3), dec!(100))
        .with(DEF, d(2024, 6, 3), dec!(100))
        .with(BOND, d(2024, 6, 3), dec!(100));
    let valuation = valuation_at(&transactions, "EUR", d(2024, 6, 3), &prices, &FakeRates::new()).unwrap();

    let taxonomy = Taxonomy::new("Classes", TaxonomyKind::AssetClass);
    let equities = TaxonomyNode::root(&taxonomy.id, "Equities");
    let core = TaxonomyNode::child(&equities, "Core");
    let defensive = TaxonomyNode::child(&equities, "Defensive");
    let bonds = TaxonomyNode::root(&taxonomy.id, "Bonds");
    let nodes = vec![equities.clone(), core.clone(), defensive.clone(), bonds.clone()];

    let assignments: Vec<Assignment> = [(CORE, &core.id), (DEF, &defensive.id), (BOND, &bonds.id)]
        .into_iter()
        .map(|(security, node)| Assignment::from(&SecurityClassification::new(security, node, Decimal::ONE)))
        .collect();
    let securities: Vec<Security> = [
        (CORE, "CORE", "Core"),
        (DEF, "DEF", "Defensive"),
        (BOND, "BOND", "Bonds"),
    ]
    .into_iter()
    .map(|(id, symbol, name)| {
        let mut s = Security::new(symbol, name, "EUR", SecurityKind::Etf);
        s.id = id.into();
        s
    })
    .collect();

    let subjects = taxonomy_subjects(&valuation, &securities, &[], &[]);
    let allocation = allocation_by_taxonomy(&subjects, &nodes, &assignments);
    let target = AllocationTarget::new("portfolio", &taxonomy.id, "Target")
        .with_weight(&equities.id, dec!(0.6))
        .with_weight(&core.id, dec!(0.8))
        .with_weight(&defensive.id, dec!(0.2))
        .with_weight(&bonds.id, dec!(0.4));

    let plan = rebalance(
        &valuation,
        &allocation,
        &target,
        &nodes,
        &assignments,
        &securities,
        &[],
        RebalanceOptions::default(),
    )
    .unwrap();

    let item = |id: &str| plan.items.iter().find(|i| i.node_id == id).unwrap().clone();

    assert_eq!(plan.total_base, dec!(10000));
    assert_eq!(item(&core.id).target_weight, dec!(0.48));
    assert_eq!(item(&core.id).target_base, dec!(4800));
    assert_eq!(item(&core.id).drift_base, dec!(-200));
    assert_eq!(item(&core.id).trades[0].quantity, dec!(-2));
    assert_eq!(item(&defensive.id).target_base, dec!(1200));
    assert_eq!(item(&defensive.id).trades[0].quantity, dec!(2));

    // Parent is a report row, not a trading participant; children hold its value.
    let parent = item(&equities.id);
    assert!(!parent.leaf);
    assert!(parent.trades.is_empty());
    assert_eq!(parent.target_base, dec!(6000));
    assert_eq!(parent.drift_base, Decimal::ZERO);
    assert_eq!(plan.off_target_base, Decimal::ZERO);

    // Level weight: target 80% versus actual 5000/6000.
    assert_eq!(item(&core.id).relative_target_weight, dec!(0.8));
    assert_eq!(item(&core.id).relative_current_weight.round_dp(4), dec!(0.8333));
}

/// Quantity step defaults and explicit overrides survive storage.
#[test]
fn quantity_step_defaults_by_kind_and_survives_storage() {
    let store = Store::open_in_memory().unwrap();

    let stock = Security::new("SAP", "SAP SE", "EUR", SecurityKind::Stock);
    assert_eq!(stock.effective_quantity_step(), Decimal::ONE);
    assert_eq!(stock.round_to_step(dec!(3.9)), dec!(3));

    let crypto = Security::new("BTC", "Bitcoin", "EUR", SecurityKind::Crypto);
    assert_eq!(crypto.effective_quantity_step(), dec!(0.00000001));
    assert_eq!(crypto.round_to_step(dec!(0.523456789)), dec!(0.52345678));

    // Trade Republic allows fractional ordinary shares, so the security overrides the default.
    let fractional =
        Security::new("ASML", "ASML Holding", "EUR", SecurityKind::Stock).with_quantity_step(dec!(0.001));
    store.save_security(&fractional).unwrap();
    let loaded = store.get_security(&fractional.id).unwrap();
    assert_eq!(loaded.quantity_step, Some(dec!(0.001)));
    assert_eq!(loaded.round_to_step(dec!(3.9876)), dec!(3.987));
}
