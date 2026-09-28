//! A commission billed in a currency neither the trade nor the portfolio is in (ADR-0064).

mod support;

use rust_decimal_macros::dec;
use sq_core::calc::PortfolioAnalytics;
use sq_core::fx::FxRate;
use sq_core::market::{DateRange, Quote};
use sq_core::model::{Account, Portfolio, Security, SecurityKind, Transaction, TransactionKind};
use sq_core::storage::Store;
use support::d;

/// 01.06 deposit 1000 EUR; 03.06 buy 5 AAPL at 100 EUR with a 9 CHF commission — a Swiss broker
/// billing its fee at home while the trade settles in euro. The rate is in the database, and
/// every day-by-day figure reads it out of the preloaded cache rather than the store, so a cache
/// built from the transaction currencies alone had no CHF in it and the whole series failed.
#[test]
fn a_fee_in_a_third_currency_is_priced_like_any_other_leg() {
    let store = Store::open_in_memory().unwrap();
    let cash = Account::deposit("Cash", "EUR");
    store.save_account(&cash).unwrap();
    let depot = Account::securities("Depot", "EUR", &cash.id);
    store.save_account(&depot).unwrap();
    let aapl = Security::new("AAPL", "Apple", "EUR", SecurityKind::Stock);
    store.save_security(&aapl).unwrap();

    for t in [
        Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            d(2024, 6, 1),
            dec!(1000),
            "EUR",
        ),
        Transaction::buy(&depot.id, &aapl.id, d(2024, 6, 3), dec!(5), dec!(100), "EUR")
            .with_fees_in(dec!(9), "CHF"),
    ] {
        store.save_transaction(&t).unwrap();
    }
    store
        .save_quotes(&[
            quote(&aapl.id, d(2024, 6, 3), dec!(100)),
            quote(&aapl.id, d(2024, 6, 5), dec!(110)),
        ])
        .unwrap();
    store
        .save_fx_rates(&[
            FxRate::new("CHF", "EUR", d(2024, 6, 1), dec!(1)),
            FxRate::new("CHF", "EUR", d(2024, 6, 3), dec!(1)),
        ])
        .unwrap();

    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash.id.clone(), depot.id.clone()]);
    let analytics = PortfolioAnalytics::new(&store, &portfolio).unwrap();
    let range = DateRange::new(d(2024, 6, 1), d(2024, 6, 5));

    // 1000 paid in, 500 of it in shares worth 550 on the 5th, and 9 CHF gone: 1000 − 9 + 50.
    let series = analytics.series(range).unwrap();
    assert_eq!(*series.total_value_base.last().unwrap(), dec!(1041));
    assert!(analytics.risk(range, 0.0).is_ok());
    assert!(analytics.position_returns(range.from, range.to).is_ok());
}

fn quote(security_id: &str, date: chrono::NaiveDate, close: rust_decimal::Decimal) -> Quote {
    Quote {
        security_id: security_id.to_string(),
        date,
        close,
        currency: "EUR".into(),
        source: "manual".into(),
    }
}
