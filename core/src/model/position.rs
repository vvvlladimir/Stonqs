use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The remainder of one specific purchase; FIFO disposes oldest lot first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    pub acquired_at: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Per-unit cost in the settlement currency (see `Position::cost_currency`).
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_per_unit: Decimal,
    /// Same, in base currency, fixed at purchase-date rate and never recomputed.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_per_unit_base: Decimal,
}

/// A security's position at a point in time — derived by replaying transactions, not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub security_id: String,
    /// Settlement currency, not the security's quote currency — see CLAUDE.md invariants.
    pub cost_currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Unsold lots, purchase order.
    pub lots: Vec<Lot>,
    /// Total cost of open lots, settlement currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis: Decimal,
    /// Same, in base currency at historical rates.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    /// Net dividends received, base currency; kept apart from realized P/L.
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_base: Decimal,
    /// Quantity held per depot. Byproduct of the transaction walk; cost basis is not split per account.
    #[serde(default)]
    pub accounts: BTreeMap<String, Decimal>,
    /// Widest decimal scale seen among this position's trade quantities. `None` = none observed.
    #[serde(default)]
    pub quantity_scale: Option<u32>,
}

impl Position {
    pub fn new(security_id: &str, cost_currency: &str) -> Self {
        Position {
            security_id: security_id.to_string(),
            cost_currency: crate::money::normalize_currency(cost_currency),
            quantity: Decimal::ZERO,
            lots: Vec::new(),
            cost_basis: Decimal::ZERO,
            cost_basis_base: Decimal::ZERO,
            realized_pnl_base: Decimal::ZERO,
            dividends_base: Decimal::ZERO,
            accounts: BTreeMap::new(),
            quantity_scale: None,
        }
    }

    /// Tracks the widest scale seen — one fractional buy proves the broker
    /// allows it, and a later round-number buy doesn't undo that.
    pub(crate) fn observe_quantity(&mut self, quantity: Decimal) {
        if quantity.is_zero() {
            return;
        }
        let seen = quantity.normalize().scale();
        self.quantity_scale = Some(self.quantity_scale.map_or(seen, |best| best.max(seen)));
    }

    pub fn observed_quantity_step(&self) -> Option<Decimal> {
        self.quantity_scale.map(|scale| Decimal::new(1, scale))
    }

    /// Zero is removed: a fully-sold account shouldn't linger as an empty row forever.
    pub(crate) fn move_on_account(&mut self, account_id: &str, delta: Decimal) {
        if delta.is_zero() {
            return;
        }
        let left = self.accounts.entry(account_id.to_string()).or_default();
        *left += delta;
        if left.is_zero() {
            self.accounts.remove(account_id);
        }
    }

    /// Exact zero comparison — `Decimal` allows it, unlike `f64` which would need an epsilon.
    pub fn is_closed(&self) -> bool {
        self.quantity.is_zero()
    }
}
