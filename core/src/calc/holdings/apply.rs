//! What each event does to the holdings.

use super::builder::HoldingsBuilder;
use super::charges::{Charges, resolve_rate};
use super::lots::{acquire, add_lot, dispose, take_lots};
use super::{CashFlow, ChargeRecord, Holdings, IncomeRecord};
use crate::error::{Error, Result};
use crate::model::{CorporateAction, CostBasisMethod, Lot, Position, Transaction, TransactionKind};
use rust_decimal::Decimal;

/// Applies a split: quantity is multiplied and per-unit cost divided, preserving total cost.
pub(super) fn corporate_action(h: &mut Holdings, action: &CorporateAction) -> Result<()> {
    let Some(position) = h.positions.get_mut(&action.security_id) else {
        // No position existed on the split date.
        return Ok(());
    };
    let factor = action.quantity_factor()?;
    if factor.is_zero() {
        return Err(Error::Invalid(format!("split {} has zero factor", action.id)));
    }
    for lot in &mut position.lots {
        lot.quantity *= factor;
        lot.cost_per_unit /= factor;
        lot.cost_per_unit_base /= factor;
    }
    position.quantity *= factor;
    // Account quantities change by the same factor; ownership does not move.
    for quantity in position.accounts.values_mut() {
        *quantity *= factor;
    }
    Ok(())
}

impl HoldingsBuilder<'_> {
    pub(super) fn transaction(&mut self, t: &Transaction) -> Result<()> {
        let HoldingsBuilder {
            holdings: h,
            base,
            rates,
            options,
            in_transit,
            paired,
        } = self;
        let (base, rates) = (base.as_str(), *rates);
        t.validate()?;
        let rate = resolve_rate(t, base, rates)?;
        let charges = Charges::of(t, rate, base, rates)?;

        // Keep cash in transaction currency; FX conversion belongs to valuation.
        let delta = t.cash_delta();
        if !delta.is_zero() {
            *h.cash.entry(t.currency.clone()).or_insert(Decimal::ZERO) += delta;
        }
        // A charge billed in another currency leaves that currency's balance, not this one's.
        for (currency, amount) in t.foreign_charge_legs() {
            *h.cash.entry(currency).or_insert(Decimal::ZERO) += amount;
        }

        // Any non-zero quantity reveals the broker's effective trading step.
        let observed = t.security_id.clone().filter(|_| !t.quantity.is_zero());

        match t.kind {
            TransactionKind::Buy => acquire(h, t, &charges, options.cost_basis),
            TransactionKind::Sell => {
                let realized = dispose(h, t, rate, &charges)?;
                h.realized_pnl_base += realized;
            }

            // Inbound delivery is an external flow at gross value; cost includes fees and taxes.
            TransactionKind::DeliveryInbound => {
                acquire(h, t, &charges, options.cost_basis);
                h.external_flows.push(CashFlow {
                    date: t.date,
                    amount_base: charges.gross_base,
                });
            }
            // Outbound delivery records P/L and a matching negative gross external flow.
            TransactionKind::DeliveryOutbound => {
                let realized = dispose(h, t, rate, &charges)?;
                h.realized_pnl_base += realized;
                h.external_flows.push(CashFlow {
                    date: t.date,
                    amount_base: -charges.gross_base,
                });
            }

            TransactionKind::SecurityTransferOut => {
                let link = t.link_id.clone().expect("validated: linked side has link_id");
                let lots = take_lots(h, t)?;
                in_transit.insert(link, lots);
            }
            TransactionKind::SecurityTransferIn => {
                let link = t.link_id.as_deref().expect("validated: linked side has link_id");
                let lots = in_transit.remove(link).ok_or_else(|| {
                    Error::Invalid(format!(
                        "security transfer {link} has no outgoing side in this scope — \
                     use DELIVERY_INBOUND if the shares came from outside the portfolio"
                    ))
                })?;
                receive_lots(h, t, lots, options.cost_basis);
            }

            TransactionKind::Dividend
            | TransactionKind::Interest
            | TransactionKind::Cashback
            | TransactionKind::Reward
            | TransactionKind::InterestCharge => income(h, t, &charges, rate),

            // Refunds reduce accumulated expenses instead of becoming income.
            TransactionKind::Fee | TransactionKind::FeeRefund => {
                let signed = charge(h, t, rate);
                h.fees_base += signed;
            }
            TransactionKind::Tax | TransactionKind::TaxRefund => {
                let signed = charge(h, t, rate);
                h.taxes_base += signed;
            }

            TransactionKind::Deposit => h.external_flows.push(CashFlow {
                date: t.date,
                amount_base: t.amount * rate,
            }),
            TransactionKind::Withdrawal => h.external_flows.push(CashFlow {
                date: t.date,
                amount_base: -t.amount * rate,
            }),
            // Only a paired transfer is internal; a lone leg crossed the boundary (`paired_links`).
            TransactionKind::TransferIn | TransactionKind::TransferOut => {
                if !t.link_id.as_deref().is_some_and(|link| paired.contains(link)) {
                    h.external_flows.push(CashFlow {
                        date: t.date,
                        amount_base: delta * rate,
                    });
                }
            }
        }

        if let Some(position) = observed.and_then(|sid| h.positions.get_mut(&sid)) {
            position.observe_quantity(t.quantity);
        }
        Ok(())
    }
}

/// Books lots that left another account under the same link as this transfer-in.
fn receive_lots(h: &mut Holdings, t: &Transaction, lots: Vec<Lot>, method: CostBasisMethod) {
    let sid = t.security_id.clone().expect("validated: transfer has security");
    let position = h
        .positions
        .entry(sid.clone())
        .or_insert_with(|| Position::new(&sid, &t.currency));
    for lot in lots {
        position.quantity += lot.quantity;
        position.move_on_account(&t.account_id, lot.quantity);
        position.cost_basis += lot.quantity * lot.cost_per_unit;
        position.cost_basis_base += lot.quantity * lot.cost_per_unit_base;
        add_lot(position, lot, method);
    }
}

/// Records one income event: a dividend, interest (earned or charged), a cashback or a reward.
fn income(h: &mut Holdings, t: &Transaction, charges: &Charges, rate: Decimal) {
    let net = charges.gross_base;
    match t.kind {
        TransactionKind::Dividend => {
            h.dividends_base += net;
            // Record every dividend; the security ID is optional.
            h.income.push(income_record(t, charges, t.amount * rate, net));
            if let Some(p) = t.security_id.as_ref().and_then(|sid| h.positions.get_mut(sid)) {
                p.dividends_base += net;
            }
        }
        TransactionKind::Interest => {
            h.interest_base += net;
            h.income.push(income_record(t, charges, t.amount * rate, net));
        }
        // Interest charges are negative income events; their components are not split here.
        TransactionKind::InterestCharge => {
            let net = -(t.amount * rate);
            h.interest_base += net;
            h.income.push(income_record(t, charges, net, net));
        }
        // Income by kind, never part of a position's dividends.
        _ => h.income.push(income_record(t, charges, t.amount * rate, net)),
    }
}

/// Builds an income event from precomputed gross and net base amounts.
fn income_record(t: &Transaction, charges: &Charges, gross_base: Decimal, net_base: Decimal) -> IncomeRecord {
    IncomeRecord {
        date: t.date,
        account_id: t.account_id.clone(),
        security_id: t.security_id.clone(),
        kind: t.kind,
        gross_base,
        taxes_base: charges.taxes_base,
        fees_base: charges.fees_base,
        net_base,
        currency: t.currency.clone(),
        // Keep the original-currency sign aligned with the base-currency amount.
        gross_in_currency: if gross_base.is_sign_negative() {
            -t.amount
        } else {
            t.amount
        },
    }
}

/// Records a signed fee or tax event and returns its signed base amount.
fn charge(h: &mut Holdings, t: &Transaction, rate: Decimal) -> Decimal {
    let refund = matches!(t.kind, TransactionKind::FeeRefund | TransactionKind::TaxRefund);
    let amount_base = if refund {
        -(t.amount * rate)
    } else {
        t.amount * rate
    };
    h.charges.push(ChargeRecord {
        date: t.date,
        account_id: t.account_id.clone(),
        security_id: t.security_id.clone(),
        kind: t.kind,
        amount_base,
        currency: t.currency.clone(),
        amount_in_currency: if refund { -t.amount } else { t.amount },
    });
    amount_base
}
