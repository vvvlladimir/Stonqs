//! Portfolio scopes: the cash account, its depot, and each visible subset.

mod support;

use rust_decimal_macros::dec;
use sq_core::calc::PortfolioAnalytics;
use sq_core::market::Quote;
use sq_core::model::{Account, Portfolio, Security, SecurityKind, Transaction, TransactionKind};
use sq_core::storage::Store;
use support::d;

/// Cash account, depot, and one security: 02.01 +1000; 03.01 10 × 90 + 1 = 901;
/// 01.03 +5; 31.12 10 × 100 = 1000; cash = 104; portfolio = 1104 EUR.
fn seeded() -> (Store, Portfolio, Account, Account) {
    let store = Store::open_in_memory().unwrap();

    let cash = Account::deposit("Savings", "EUR");
    store.save_account(&cash).unwrap();
    let depot = Account::securities("TR", "EUR", &cash.id);
    store.save_account(&depot).unwrap();

    let etf = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf);
    store.save_security(&etf).unwrap();
    store
        .save_quotes(&[Quote {
            security_id: etf.id.clone(),
            date: d(2024, 12, 31),
            close: dec!(100),
            currency: "EUR".into(),
            source: "manual".into(),
        }])
        .unwrap();

    for t in [
        Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            d(2024, 1, 2),
            dec!(1000),
            "EUR",
        ),
        Transaction::buy(&depot.id, &etf.id, d(2024, 1, 3), dec!(10), dec!(90), "EUR").with_fees(dec!(1)),
        Transaction::dividend(&depot.id, &etf.id, d(2024, 3, 1), dec!(5), "EUR"),
    ] {
        store.save_transaction(&t).unwrap();
    }

    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash.id.clone(), depot.id.clone()]);
    (store, portfolio, cash, depot)
}

/// The full portfolio keeps its pre-scope result.
#[test]
fn the_whole_portfolio_adds_securities_and_cash() {
    let (store, portfolio, _, _) = seeded();
    let valuation = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .valuation_at(d(2024, 12, 31))
        .unwrap();

    assert_eq!(valuation.securities_value_base, dec!(1000));
    assert_eq!(valuation.cash_base, dec!(104));
    assert_eq!(valuation.total_value_base, dec!(1104));
    assert_eq!(valuation.dividends_base, dec!(5));
}

/// Depot alone has securities but no cash; cost = 901 EUR and unrealized = 1000 − 901 = 99 EUR.
#[test]
fn a_depot_alone_holds_securities_and_no_cash() {
    let (store, portfolio, _, depot) = seeded();
    let valuation = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .valuation_at(d(2024, 12, 31))
        .unwrap();

    assert_eq!(valuation.cash_base, dec!(0));
    assert_eq!(valuation.securities_value_base, dec!(1000));
    assert_eq!(valuation.total_value_base, dec!(1000));
    assert_eq!(valuation.cost_basis_base, dec!(901));
    assert_eq!(valuation.unrealized_pnl_base, dec!(99));
    assert_eq!(valuation.dividends_base, dec!(0));
}

/// Cash account alone has no securities and matches the statement balance.
#[test]
fn a_deposit_account_alone_holds_cash_and_no_securities() {
    let (store, portfolio, cash, _) = seeded();
    let valuation = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&cash.id))
        .valuation_at(d(2024, 12, 31))
        .unwrap();

    assert!(valuation.positions.is_empty());
    assert_eq!(valuation.securities_value_base, dec!(0));
    assert_eq!(valuation.cash_base, dec!(104));
    assert_eq!(valuation.total_value_base, dec!(104));
    assert_eq!(valuation.dividends_base, dec!(5));
}

/// Account balances are statement facts; a scope only selects visible accounts.
#[test]
fn cash_balances_follow_the_settlement_account() {
    let (store, portfolio, cash, depot) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).unwrap();

    let all = analytics.cash_balances(d(2024, 12, 31)).unwrap();
    assert_eq!(all[&cash.id][&"EUR".to_string()], dec!(104));
    assert!(!all.contains_key(&depot.id));

    // A depot scope has no cash row; this is absence, not a zero balance.
    let only_depot = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .cash_balances(d(2024, 12, 31))
        .unwrap();
    assert!(only_depot.is_empty());
}

/// Depot return treats the security transfer as the external flow: TWR = 1000 / 901 − 1 = 10.99%.
#[test]
fn a_depot_alone_returns_the_price_performance_of_its_securities() {
    let (store, portfolio, _, depot) = seeded();
    let twr = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .twr(d(2024, 1, 1), d(2024, 12, 31))
        .unwrap();

    let expected = dec!(1000) / dec!(901) - dec!(1);
    assert!(
        (twr - expected).abs() < dec!(0.0001),
        "depot TWR {twr} instead of {expected}"
    );
}

/// Depot XIRR converges because the scope has a flow and terminal value.
#[test]
fn a_depot_alone_has_a_solvable_irr() {
    let (store, portfolio, _, depot) = seeded();
    let irr = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .xirr(d(2024, 12, 31))
        .unwrap();

    // Invested 901 for almost a year and received 1000: slightly above 11%.
    assert!(irr > dec!(0.10) && irr < dec!(0.13), "depot IRR {irr}");
}

/// A depot-only lens still traded. The buy is 10 × 90 + 1 commission = 901 EUR, and the lens
/// turns it into a delivery only because the cash that paid for it sits on the other account.
/// Turnover must therefore be the same 901 the whole portfolio reports, or the depot would
/// claim a commission it never paid for a trade it never made.
#[test]
fn a_depot_alone_counts_the_trade_the_lens_turned_into_a_delivery() {
    let (store, portfolio, _, depot) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).unwrap();

    let whole = analytics.trading_volume(d(2024, 1, 1), d(2024, 12, 31)).unwrap();
    assert_eq!(whole.bought_base, dec!(901));
    assert_eq!(whole.sold_base, dec!(0));
    assert_eq!(whole.volume_base, dec!(901));
    assert_eq!(whole.trades, 1);

    let only_depot = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .trading_volume(d(2024, 1, 1), d(2024, 12, 31))
        .unwrap();
    assert_eq!(only_depot.bought_base, dec!(901));
    assert_eq!(only_depot.volume_base, dec!(901));
    assert_eq!(only_depot.trades, 1);
}

/// The other half of the same rewrite: from the deposit account the shares never arrived, so
/// the 901 that left it is a withdrawal, not a purchase. A cash lens trades nothing.
#[test]
fn a_deposit_account_alone_trades_nothing() {
    let (store, portfolio, cash, _) = seeded();
    let volume = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&cash.id))
        .trading_volume(d(2024, 1, 1), d(2024, 12, 31))
        .unwrap();

    assert_eq!(volume.volume_base, dec!(0));
    assert_eq!(volume.trades, 0);
}

/// A genuine delivery is still not a trade: shares handed over by another broker cost no
/// commission and were nobody's decision here, whatever the lens is.
#[test]
fn a_real_delivery_is_never_counted_as_volume() {
    let (store, portfolio, _, depot) = seeded();
    let etf = store.list_securities().unwrap().into_iter().next().unwrap();
    store
        .save_transaction(&Transaction::delivery_inbound(
            &depot.id,
            &etf.id,
            d(2024, 6, 1),
            dec!(5),
            dec!(475),
            "EUR",
        ))
        .unwrap();

    let volume = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .scoped_to(std::slice::from_ref(&depot.id))
        .trading_volume(d(2024, 1, 1), d(2024, 12, 31))
        .unwrap();

    assert_eq!(volume.volume_base, dec!(901));
    assert_eq!(volume.trades, 1);
}
