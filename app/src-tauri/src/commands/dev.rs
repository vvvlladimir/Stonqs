use crate::error::{UiError, UiResult};
#[cfg(debug_assertions)]
use crate::state::AppState;
#[cfg(debug_assertions)]
use tauri::State;

#[cfg(debug_assertions)]
#[tauri::command]
pub fn dev_seed_demo(state: State<AppState>) -> UiResult<()> {
    seed(&state)
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn dev_seed_demo() -> UiResult<()> {
    Err(UiError::invalid("demo data is only available in a debug build"))
}

/// What the alert simulator does to one rule.
#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevAlertStep {
    /// Writes today's close one percent past the level, on the side the price is not on; a date
    /// rule is moved to today.
    Cross,
    /// Deletes the simulated quotes of the rule's instrument, so the price returns to the real one.
    Reset,
}

/// Source of simulated quotes, so `Reset` removes exactly those and a refresh overwrites them.
#[cfg(debug_assertions)]
const DEV_SOURCE: &str = "dev";

/// Moves a real quote rather than faking a crossing: the check that runs after it is the same one
/// a refresh runs, so the log, the dot and the notification are what a real move would produce.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn dev_alert_simulate(
    app: tauri::AppHandle,
    state: State<AppState>,
    alert_id: String,
    step: DevAlertStep,
) -> UiResult<usize> {
    use crate::commands::alerts::{check_alerts, today};
    use sq_core::prelude::*;

    let today = today();
    let logged = {
        let store = state.store()?;
        // The simulated close is placed against the side the real quotes leave the rule on.
        check_alerts(&store, today)?;
        let mut alert = store.get_alert(&alert_id)?;
        match (step, alert.kind) {
            (DevAlertStep::Cross, AlertKind::Price) => {
                let close = sq_core::calc::crossing_close(&alert)
                    .ok_or_else(|| UiError::invalid("the alert has no level"))?;
                store.save_quotes(&[Quote {
                    security_id: alert.security_id.clone(),
                    date: today,
                    close,
                    currency: alert.currency.clone().unwrap_or_default(),
                    source: DEV_SOURCE.into(),
                }])?;
            }
            (DevAlertStep::Cross, AlertKind::DateReached) => {
                alert.date = Some(today);
                alert.checked_through = None;
                store.save_alert(&alert)?;
            }
            (DevAlertStep::Reset, _) => {
                store.delete_quotes_from(&alert.security_id, DEV_SOURCE)?;
            }
        }
        check_alerts(&store, today)?
    };
    crate::events::emit_changed(&app, "quotes")?;
    crate::events::emit_changed(&app, "alerts")?;
    Ok(logged)
}

#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn dev_alert_simulate() -> UiResult<usize> {
    Err(UiError::invalid(
        "the alert simulator is only available in a debug build",
    ))
}

#[cfg(debug_assertions)]
fn seed(state: &State<AppState>) -> UiResult<()> {
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
