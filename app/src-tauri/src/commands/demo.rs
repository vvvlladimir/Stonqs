//! The demo portfolio offered to a new profile.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use tauri::State;

/// Fills an empty portfolio with a small two-account, two-instrument history.
///
/// Ships in every build, not just a debug one: somebody who has just downloaded this is not going
/// to hand their broker statement to an application they have never seen, and a portfolio they can
/// click through is the only honest way to show what it does. It refuses once the portfolio has
/// an account, so it can never write over real data.
#[tauri::command]
pub fn demo_seed(state: State<AppState>) -> UiResult<()> {
    let state = &state;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use sq_core::prelude::*;

    fn day(month: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, month, d).expect("a valid demo date")
    }

    {
        let store = state.store()?;
        let mut portfolio = state.portfolio()?;

        if !portfolio.account_ids.is_empty() {
            return Err(UiError::invalid("the portfolio is no longer empty"));
        }

        let ib_cash = Account::deposit("Global broker · cash", "USD");
        let tr_cash = Account::deposit("Euro broker · cash", "EUR");
        store.save_account(&ib_cash)?;
        store.save_account(&tr_cash)?;
        let ib = Account::securities("Global broker", "USD", &ib_cash.id);
        let tr = Account::securities("Euro broker", "EUR", &tr_cash.id);
        store.save_account(&ib)?;
        store.save_account(&tr)?;

        let apple =
            Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("yahoo", "AAPL");
        let world = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf)
            .with_quantity_step(dec!(0.001));
        store.save_security(&apple)?;
        store.save_security(&world)?;

        portfolio.account_ids = vec![
            ib_cash.id.clone(),
            tr_cash.id.clone(),
            ib.id.clone(),
            tr.id.clone(),
        ];
        store.save_portfolio(&portfolio)?;

        for tx in [
            Transaction::cash(
                &ib_cash.id,
                TransactionKind::Deposit,
                day(6, 1),
                dec!(2000),
                "USD",
            )
            .with_fx_rate(dec!(0.92)),
            Transaction::buy(&ib.id, &apple.id, day(6, 5), dec!(5), dec!(195), "USD")
                .with_fees(dec!(1))
                .with_fx_rate(dec!(0.92)),
            Transaction::buy(&ib.id, &apple.id, day(8, 12), dec!(5), dec!(216), "USD")
                .with_fees(dec!(1))
                .with_fx_rate(dec!(0.91)),
            Transaction::dividend(&ib.id, &apple.id, day(8, 15), dec!(12.50), "USD")
                .with_taxes(dec!(1.88))
                .with_fx_rate(dec!(0.91)),
            Transaction::sell(&ib.id, &apple.id, day(11, 4), dec!(4), dec!(222), "USD")
                .with_fees(dec!(1))
                .with_fx_rate(dec!(0.92)),
            Transaction::cash(
                &tr_cash.id,
                TransactionKind::Deposit,
                day(6, 1),
                dec!(3000),
                "EUR",
            ),
            Transaction::buy(&tr.id, &world.id, day(6, 10), dec!(30), dec!(85), "EUR").with_fees(dec!(1)),
        ] {
            store.save_transaction(&tx)?;
        }

        let mut quotes = Vec::new();
        for (security, currency, series) in [
            (
                &apple,
                "USD",
                vec![
                    (day(6, 5), dec!(195)),
                    (day(8, 12), dec!(216)),
                    (day(11, 4), dec!(222)),
                    (day(12, 31), dec!(250)),
                ],
            ),
            (
                &world,
                "EUR",
                vec![
                    (day(6, 10), dec!(85)),
                    (day(8, 12), dec!(90)),
                    (day(11, 4), dec!(95)),
                    (day(12, 31), dec!(100)),
                ],
            ),
        ] {
            for (date, close) in series {
                quotes.push(Quote {
                    security_id: security.id.clone(),
                    date,
                    close,
                    currency: currency.into(),
                    source: "manual".into(),
                });
            }
        }
        store.save_quotes(&quotes)?;

        store.save_fx_rates(&[
            FxRate::new("USD", "EUR", day(6, 1), dec!(0.92)),
            FxRate::new("USD", "EUR", day(8, 12), dec!(0.91)),
            FxRate::new("USD", "EUR", day(11, 4), dec!(0.92)),
            FxRate::new("USD", "EUR", day(12, 31), dec!(0.95)),
        ])?;
    }

    state.reload_portfolio()
}
