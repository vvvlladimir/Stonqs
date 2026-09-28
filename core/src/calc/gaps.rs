//! Disposals of shares the ledger never received: holes, not short positions. The engine bridges
//! each with an implied delivery at the disposal's price (ADR-0089).

use super::holdings::{Event, ordered_events};
use crate::error::Result;
use crate::model::{CorporateAction, Transaction, TransactionKind};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One disposal that takes more than was held at that moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantityGap {
    pub transaction_id: String,
    pub date: NaiveDate,
    pub account_id: String,
    pub security_id: String,
    pub kind: TransactionKind,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// What the ledger held just before the disposal; never negative.
    #[serde(with = "rust_decimal::serde::str")]
    pub held: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub missing: Decimal,
}

/// Every disposal exceeding its position, in the order the holdings builder applies them.
pub fn quantity_gaps(transactions: &[Transaction], actions: &[CorporateAction]) -> Result<Vec<QuantityGap>> {
    let mut held: HashMap<&str, Decimal> = HashMap::new();
    let mut gaps = Vec::new();
    for event in ordered_events(transactions, actions) {
        match event {
            Event::Action(action) => {
                if let Some(q) = held.get_mut(action.security_id.as_str()) {
                    // Rounded exactly as the holdings builder rounds it, or a disposal of the
                    // whole position is a gap to one of the two and not to the other.
                    *q = crate::money::fit_quantity(*q * action.quantity_factor()?);
                }
            }
            Event::Tx(t) => {
                let Some(sid) = t.security_id.as_deref() else {
                    continue;
                };
                let q = held.entry(sid).or_insert(Decimal::ZERO);
                match t.kind.quantity_sign() {
                    1 => *q += t.quantity,
                    -1 => {
                        if t.quantity > *q {
                            gaps.push(QuantityGap {
                                transaction_id: t.id.clone(),
                                date: t.date,
                                account_id: t.account_id.clone(),
                                security_id: sid.to_string(),
                                kind: t.kind,
                                quantity: t.quantity,
                                held: *q,
                                missing: t.quantity - *q,
                            });
                            *q = Decimal::ZERO;
                        } else {
                            *q -= t.quantity;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(gaps)
}

/// The ledger with an implied inbound delivery before each gap, at the disposal's price: nil
/// result, value in as a flow. Never stored.
pub(crate) fn bridge_gaps(
    transactions: Vec<Transaction>,
    actions: &[CorporateAction],
) -> Result<Vec<Transaction>> {
    let gaps = quantity_gaps(&transactions, actions)?;
    if gaps.is_empty() {
        return Ok(transactions);
    }
    let by_id: HashMap<&str, &QuantityGap> = gaps.iter().map(|g| (g.transaction_id.as_str(), g)).collect();
    let mut out = Vec::with_capacity(transactions.len() + gaps.len());
    for t in &transactions {
        if let Some(gap) = by_id.get(t.id.as_str()) {
            out.push(implied_delivery(t, gap));
        }
        out.push(t.clone());
    }
    Ok(out)
}

fn implied_delivery(disposal: &Transaction, gap: &QuantityGap) -> Transaction {
    let mut t = Transaction::delivery_inbound(
        &disposal.account_id,
        &gap.security_id,
        disposal.date,
        gap.missing,
        gap.missing * disposal.price,
        &disposal.currency,
    );
    t.id = format!("implied:{}", disposal.id);
    t.fx_rate_to_base = disposal.fx_rate_to_base;
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 4, day).unwrap()
    }

    #[test]
    fn a_sale_of_shares_never_received_is_one_gap_of_the_whole_quantity() {
        let txs = vec![
            Transaction::buy("a", "XLM", d(1), dec!(10), dec!(1), "EUR"),
            Transaction::sell("a", "XLM", d(2), dec!(4), dec!(1), "EUR"),
            // 6 held, 10 sold: 4 missing.
            Transaction::sell("a", "XLM", d(3), dec!(10), dec!(1), "EUR"),
            // Nothing held any more: the whole 5 is missing, the earlier gap is not counted twice.
            Transaction::sell("a", "XLM", d(4), dec!(5), dec!(1), "EUR"),
        ];
        let gaps = quantity_gaps(&txs, &[]).unwrap();
        assert_eq!(gaps.len(), 2);
        assert_eq!((gaps[0].held, gaps[0].missing), (dec!(6), dec!(4)));
        assert_eq!((gaps[1].held, gaps[1].missing), (dec!(0), dec!(5)));
    }

    #[test]
    fn a_same_day_receipt_covers_a_sale_stored_before_it() {
        // Storage order within a day is random; the receipt is applied first regardless.
        let mut receipt = Transaction::delivery_inbound("a", "XLM", d(19), dec!(580), dec!(87), "EUR");
        receipt.id = "z".into();
        let mut sale = Transaction::sell("a", "XLM", d(19), dec!(580), dec!(0.1458), "EUR");
        sale.id = "a".into();
        assert!(quantity_gaps(&[sale, receipt], &[]).unwrap().is_empty());
    }
}
