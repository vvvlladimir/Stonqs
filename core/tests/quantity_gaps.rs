//! A sale of shares the ledger never received: reported, bridged, and no longer fatal.

mod support;

use rust_decimal_macros::dec;
use sq_core::calc::{PortfolioAnalytics, build_holdings};
use sq_core::market::DateRange;
use sq_core::model::{Account, Portfolio, Security, SecurityKind, Transaction, TransactionKind};
use sq_core::storage::Store;
use support::{FakeRates, d};

/// 01.04 deposit 100 EUR; 19.04 sell 580 XLM × 0.15 = 87 EUR — the crypto transfer that
/// brought them in is not in the ledger.
fn seeded() -> (Store, Portfolio, Security) {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Cash", "EUR");
    store.save_account(&cash).unwrap();
    let depot = Account::securities("TR", "EUR", &cash.id);
    store.save_account(&depot).unwrap();
    let xlm = Security::new("XLM-EUR", "Stellar", "EUR", SecurityKind::Crypto);
    store.save_security(&xlm).unwrap();
    for t in [
        Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            d(2026, 4, 1),
            dec!(100),
            "EUR",
        ),
        Transaction::sell(&depot.id, &xlm.id, d(2026, 4, 19), dec!(580), dec!(0.15), "EUR"),
    ] {
        store.save_transaction(&t).unwrap();
    }
    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash.id.clone(), depot.id.clone()]);
    (store, portfolio, xlm)
}

/// The pure builder still refuses the ledger: it is inconsistent, and a caller that did not ask
/// for a bridge must not get one.
#[test]
fn the_holdings_builder_still_refuses_it() {
    let (store, portfolio, _) = seeded();
    let txs = store
        .transactions_for_accounts(&portfolio.account_ids, None)
        .unwrap();
    assert!(build_holdings(&txs, "EUR", &FakeRates::new()).is_err());
}

/// Held 0, sold 580: one gap of 580, named by instrument and date.
#[test]
fn the_gap_is_reported() {
    let (store, portfolio, xlm) = seeded();
    let gaps = PortfolioAnalytics::new(&store, &portfolio)
        .unwrap()
        .quantity_gaps()
        .unwrap();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].security_id, xlm.id);
    assert_eq!(gaps[0].date, d(2026, 4, 19));
    assert_eq!((gaps[0].held, gaps[0].missing), (dec!(0), dec!(580)));
}

/// Bridged at the sale's own price: 580 × 0.15 = 87 arrives as a flow and leaves as proceeds,
/// so the result is 0, cash is 100 + 87 = 187, and the period's return is 187 / (100 + 87) − 1 = 0.
#[test]
fn the_figures_are_computed_around_it() {
    let (store, portfolio, _) = seeded();
    let analytics = PortfolioAnalytics::new(&store, &portfolio).unwrap();

    let valuation = analytics.valuation_at(d(2026, 4, 30)).unwrap();
    assert_eq!(valuation.cash_base, dec!(187));
    assert_eq!(valuation.realized_pnl_base, dec!(0));

    let twr = analytics.twr(d(2026, 4, 1), d(2026, 4, 30)).unwrap();
    assert_eq!(twr, dec!(0));

    let series = analytics
        .series(DateRange::new(d(2026, 4, 1), d(2026, 4, 30)))
        .unwrap();
    assert_eq!(
        series.external_flow_base.iter().sum::<rust_decimal::Decimal>(),
        dec!(187)
    );
}
