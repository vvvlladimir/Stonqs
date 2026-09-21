//! JSON wire format shared with the frontend.

mod ai;
mod app;
mod dashboard;
mod import;
mod inflation;
mod ledger;
mod market;
mod performance;
mod positions;
mod reports;
mod taxonomy;

use rust_decimal_macros::dec;
use serde_json::Value;
use sq_app_lib::commands::dashboard::{AccountRef, AppStatus, CashBalance, DashboardData, SecurityRef};
use sq_core::calc::{PortfolioValuation, PositionValuation};
use sq_core::model::AccountKind;

fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
    keys.sort();
    keys
}

fn sample() -> DashboardData {
    let position = PositionValuation {
        security_id: "sec-1".into(),
        // Quote and cost currencies intentionally differ.
        currency: "USD".into(),
        cost_currency: "EUR".into(),
        quantity: dec!(6),
        price: dec!(250),
        fx_rate: dec!(0.95),
        cost_fx_rate: dec!(1),
        market_value: dec!(1500),
        market_value_base: dec!(1425),
        cost_basis: dec!(1225.58),
        cost_basis_base: dec!(1163.30),
        unrealized_pnl_base: dec!(261.70),
        currency_gain_base: dec!(0),
        realized_pnl_base: dec!(97.70),
        // Fractional quantity is inferred from transaction history.
        observed_quantity_step: Some(dec!(0.000001)),
    };

    DashboardData {
        valuation: PortfolioValuation {
            date: chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            base_currency: "EUR".into(),
            positions: vec![position],
            securities_value_base: dec!(4425),
            cash_base: dec!(1247.58),
            total_value_base: dec!(5672.58),
            cost_basis_base: dec!(3714.29),
            unrealized_pnl_base: dec!(710.70),
            currency_gain_base: dec!(0),
            realized_pnl_base: dec!(97.70),
            realized_currency_gain_base: dec!(0),
            dividends_base: dec!(9.66),
            interest_base: dec!(0),
            fees_base: dec!(2.92),
            taxes_base: dec!(1.71),
        },
        securities: vec![SecurityRef {
            id: "sec-1".into(),
            symbol: "AAPL".into(),
            name: "Apple Inc.".into(),
            currency: "USD".into(),
        }],
        accounts: vec![AccountRef {
            id: "acc-1".into(),
            name: "Bank".into(),
            kind: AccountKind::Deposit,
            currency: "EUR".into(),
        }],
        cash: vec![CashBalance {
            account_id: "acc-1".into(),
            currency: "EUR".into(),
            amount: dec!(1247.58),
        }],
    }
}
