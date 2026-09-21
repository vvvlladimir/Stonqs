//! The same position read under both cost-basis methods at once.
//!
//! The portfolio answers to one method ([`crate::model::CostBasisMethod`]), and every figure
//! elsewhere in `calc` uses it. This module exists because the two methods disagree about one
//! thing only — *which* shares a sale took — and that disagreement is worth showing rather than
//! deciding for the user: FIFO sells the oldest shares, average cost sells an average share, so
//! after a partial sale what is left has a different purchase value under each.
//!
//! Neither is more correct; they are different tax conventions. What they cannot disagree about
//! is the end: once a position is closed, realised plus unrealised is the same total either way.

use super::{Holdings, PortfolioValuation};
use crate::money::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Purchase value and result of one instrument under one method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostBasisFigures {
    /// What the shares still held cost, in the currency they were paid for.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_basis_base: Decimal,
    /// Purchase price per share; zero when nothing is held.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_per_unit: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_per_unit_base: Decimal,
    /// Market value minus what is left of the cost — the method moves the cost, never the value.
    #[serde(with = "rust_decimal::serde::str")]
    pub unrealized_pnl_base: Decimal,
    /// Result of the disposals made up to the date, under this method.
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
}

/// One instrument under both methods, against one market value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostBasisRow {
    pub security_id: String,
    /// Currency the shares were paid for; `cost_basis` and `cost_per_unit` are in it.
    pub cost_currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub market_value_base: Decimal,
    pub fifo: CostBasisFigures,
    /// Moving average: one lot whose price is re-averaged at every purchase.
    pub average: CostBasisFigures,
}

impl CostBasisRow {
    /// What the choice of method is worth on this row today: purchase value under FIFO minus
    /// under moving average. Zero for a position that was never partly sold.
    pub fn spread_base(&self) -> Decimal {
        self.fifo.cost_basis_base - self.average.cost_basis_base
    }
}

/// Reads every open position of `valuation` under both methods.
///
/// The two holdings must be built from the same transactions through the same date and differ
/// only in their [`crate::calc::HoldingsOptions::cost_basis`] — quantities are method-free, which
/// is why one valuation serves both and no price is read twice.
///
/// Only open positions are listed: a closed one has no purchase value left to disagree about,
/// and its realised total is the same under either method.
pub fn compare_cost_basis(
    fifo: &Holdings,
    average: &Holdings,
    valuation: &PortfolioValuation,
) -> Vec<CostBasisRow> {
    valuation
        .positions
        .iter()
        .map(|p| CostBasisRow {
            security_id: p.security_id.clone(),
            cost_currency: p.cost_currency.clone(),
            quantity: p.quantity,
            market_value_base: p.market_value_base,
            fifo: figures(fifo, &p.security_id, p.quantity, p.market_value_base),
            average: figures(average, &p.security_id, p.quantity, p.market_value_base),
        })
        .collect()
}

fn figures(
    holdings: &Holdings,
    security_id: &str,
    quantity: Decimal,
    market_value_base: Decimal,
) -> CostBasisFigures {
    let Some(position) = holdings.positions.get(security_id) else {
        return CostBasisFigures::default();
    };
    let unrealized = market_value_base - position.cost_basis_base;
    // A position closed and reopened can read zero here; dividing by it would invent a price.
    let per_unit = |total: Decimal| {
        if quantity.is_zero() {
            Decimal::ZERO
        } else {
            total / quantity
        }
    };
    CostBasisFigures {
        cost_basis: position.cost_basis,
        cost_basis_base: position.cost_basis_base,
        cost_per_unit: per_unit(position.cost_basis),
        cost_per_unit_base: per_unit(position.cost_basis_base),
        unrealized_pnl_base: unrealized,
        realized_pnl_base: position.realized_pnl_base,
    }
}
