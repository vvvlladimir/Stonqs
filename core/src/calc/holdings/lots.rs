//! Lots in and out of a position.

use super::charges::Charges;
use super::{Holdings, RealizedGain};
use crate::error::{Error, Result};
use crate::model::{CostBasisMethod, Lot, Position, Transaction};
use rust_decimal::Decimal;

/// Adds a lot using FIFO or a single weighted-average lot.
pub(super) fn add_lot(position: &mut Position, lot: Lot, method: CostBasisMethod) {
    match method {
        CostBasisMethod::Fifo => position.lots.push(lot),
        CostBasisMethod::AverageCost => match position.lots.first_mut() {
            None => position.lots.push(lot),
            Some(existing) => {
                let total = existing.quantity + lot.quantity;
                if total.is_zero() {
                    return;
                }
                existing.cost_per_unit =
                    (existing.quantity * existing.cost_per_unit + lot.quantity * lot.cost_per_unit) / total;
                existing.cost_per_unit_base = (existing.quantity * existing.cost_per_unit_base
                    + lot.quantity * lot.cost_per_unit_base)
                    / total;
                existing.quantity = total;
                existing.acquired_at = existing.acquired_at.min(lot.acquired_at);
            }
        },
    }
}

/// Acquires securities with known cost (purchase or inbound delivery).
pub(super) fn acquire(h: &mut Holdings, t: &Transaction, charges: &Charges, method: CostBasisMethod) {
    let sid = t
        .security_id
        .clone()
        .expect("validated: acquisition has security");
    let position = h
        .positions
        .entry(sid.clone())
        .or_insert_with(|| Position::new(&sid, &t.currency));

    // Total cost is trade amount plus fees and taxes, whichever currency each was paid in.
    let cost = charges.gross_in_currency;
    let cost_base = charges.gross_base;

    position.quantity += t.quantity;
    position.move_on_account(&t.account_id, t.quantity);
    position.cost_basis += cost;
    position.cost_basis_base += cost_base;
    add_lot(
        position,
        Lot {
            acquired_at: t.date,
            quantity: t.quantity,
            cost_per_unit: cost / t.quantity,
            // Capture the transaction FX rate; lot cost never revalues.
            cost_per_unit_base: cost_base / t.quantity,
        },
        method,
    );
}

/// Removes `quantity` from a position and returns the consumed lots.
pub(super) fn take_lots(h: &mut Holdings, t: &Transaction) -> Result<Vec<Lot>> {
    let sid = t.security_id.clone().expect("validated: disposal has security");
    let position = h
        .positions
        .get_mut(&sid)
        .ok_or_else(|| Error::Invalid(format!("disposal of {sid} with no open position")))?;

    if t.quantity > position.quantity {
        return Err(Error::Invalid(format!(
            "disposal of {} exceeds position {} for {sid} on {}",
            t.quantity, position.quantity, t.date
        )));
    }

    // Consume from the queue head; FIFO uses oldest lots, average cost has one lot.
    let mut remaining = t.quantity;
    let mut taken = Vec::new();
    while remaining > Decimal::ZERO {
        // The check above is against the position, and a split's rounding can still leave the
        // lots a fraction short of it. What is left over then is dust, never a short sale.
        let Some(lot) = position.lots.first_mut() else {
            break;
        };
        let take = remaining.min(lot.quantity);
        taken.push(Lot {
            acquired_at: lot.acquired_at,
            quantity: take,
            cost_per_unit: lot.cost_per_unit,
            cost_per_unit_base: lot.cost_per_unit_base,
        });
        lot.quantity -= take;
        remaining -= take;
        if lot.quantity.is_zero() {
            position.lots.remove(0);
        }
    }

    let removed_cost: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit).sum();
    let removed_cost_base: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit_base).sum();
    position.quantity -= t.quantity;
    // Attribute the disposal to its transaction account; account totals remain auditable.
    position.move_on_account(&t.account_id, -t.quantity);
    position.cost_basis -= removed_cost;
    position.cost_basis_base -= removed_cost_base;
    Ok(taken)
}

/// Disposes securities and records realized P/L in base currency.
pub(super) fn dispose(
    h: &mut Holdings,
    t: &Transaction,
    rate: Decimal,
    charges: &Charges,
) -> Result<Decimal> {
    let sid = t.security_id.clone().expect("validated: disposal has security");
    let cost_currency = h.positions.get(&sid).map(|p| p.cost_currency.clone());
    let taken = take_lots(h, t)?;
    let removed_cost_base: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit_base).sum();
    let removed_cost: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit).sum();

    let proceeds_base = charges.gross_base;
    // Realized P/L is base-currency proceeds minus historical base cost.
    let realized = proceeds_base - removed_cost_base;

    // The rate's share of the result (ADR-0028); none when sale and purchase currencies differ.
    let currency_gain_base = match cost_currency {
        Some(currency) if currency == t.currency => removed_cost * rate - removed_cost_base,
        _ => Decimal::ZERO,
    };

    h.realized.push(RealizedGain {
        date: t.date,
        security_id: sid.clone(),
        kind: t.kind,
        quantity: t.quantity,
        // Keep gross proceeds, fees, and taxes separate for reporting.
        proceeds_base: t.amount * rate,
        fees_base: charges.fees_base,
        taxes_base: charges.taxes_base,
        cost_base: removed_cost_base,
        cost_in_currency: removed_cost,
        gain_base: realized,
        currency_gain_base,
        lots: taken,
    });
    h.realized_currency_gain_base += currency_gain_base;
    if let Some(p) = h.positions.get_mut(&sid) {
        p.realized_pnl_base += realized;
    }
    Ok(realized)
}
