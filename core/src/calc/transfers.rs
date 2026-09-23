//! Finding the other half of a move that crossed the portfolio boundary on paper only.
//!
//! Two brokers export separately, so a withdrawal at one and a deposit at the other arrive as
//! two unrelated rows. `calc` reads both as money entering and leaving the portfolio, which is
//! exactly what TWR, XIRR and every capital figure divide by — the same reason a leg with no
//! partner is never treated as internal on its own (`.claude/rules/import.md`).
//!
//! Matching is a *suggestion*, never a decision: two amounts agreeing is not proof that one
//! payment is the other. The caller confirms a pair before anything is written.

use crate::model::{Transaction, TransactionKind};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;
use std::collections::HashSet;

/// How far apart the two legs of one move may settle. Two business days plus a weekend is what
/// a SEPA transfer between brokers actually takes.
const MAX_DAYS: i64 = 5;

/// How far the amounts may differ, as a share of the larger: an intermediary bank fee comes off
/// the amount in flight, so the two legs of one move genuinely are not equal.
const AMOUNT_TOLERANCE: Decimal = dec!(0.01);

/// Two stored operations that look like the two halves of one move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransferPair {
    /// The leg money left on.
    pub out_id: String,
    /// The leg money arrived on.
    pub in_id: String,
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_out: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_in: Decimal,
    pub date_out: String,
    pub date_in: String,
    pub account_out: String,
    pub account_in: String,
    /// Days between the two legs; the caller ranks by it.
    pub days_apart: i64,
}

/// Whether an operation is the kind of thing that could be one leg of a move: money in or out,
/// no instrument attached, and not already linked to a partner.
fn is_candidate(t: &Transaction) -> bool {
    t.link_id.is_none()
        && t.security_id.is_none()
        && !t.amount.is_zero()
        && matches!(
            t.kind,
            TransactionKind::Deposit
                | TransactionKind::Withdrawal
                | TransactionKind::TransferIn
                | TransactionKind::TransferOut
        )
}

fn close_enough(a: Decimal, b: Decimal) -> bool {
    let larger = a.abs().max(b.abs());
    larger > Decimal::ZERO && (a.abs() - b.abs()).abs() <= larger * AMOUNT_TOLERANCE
}

/// Unlinked outgoing and incoming operations that match in currency, amount and date, on two
/// different accounts. Each operation appears in at most one suggestion — the closest in time —
/// so confirming the whole list cannot link one leg twice.
pub fn transfer_candidates(transactions: &[Transaction]) -> Vec<TransferPair> {
    let legs: Vec<&Transaction> = transactions.iter().filter(|t| is_candidate(t)).collect();
    let (out, into): (Vec<&Transaction>, Vec<&Transaction>) =
        legs.into_iter().partition(|t| t.kind.cash_sign() < 0);

    let mut pairs: Vec<TransferPair> = Vec::new();
    for left in &out {
        for right in &into {
            if left.account_id == right.account_id || left.currency != right.currency {
                continue;
            }
            let days = (right.date - left.date).num_days().abs();
            if days > MAX_DAYS || !close_enough(left.amount, right.amount) {
                continue;
            }
            pairs.push(TransferPair {
                out_id: left.id.clone(),
                in_id: right.id.clone(),
                currency: left.currency.clone(),
                amount_out: left.amount,
                amount_in: right.amount,
                date_out: left.date.to_string(),
                date_in: right.date.to_string(),
                account_out: left.account_id.clone(),
                account_in: right.account_id.clone(),
                days_apart: days,
            });
        }
    }

    // Closest in time first, so a leg with two possible partners is offered the likelier one and
    // the other suggestion drops out with it.
    pairs.sort_by(|a, b| (a.days_apart, &a.out_id, &a.in_id).cmp(&(b.days_apart, &b.out_id, &b.in_id)));
    let mut used: HashSet<String> = HashSet::new();
    pairs.retain(|pair| {
        if used.contains(&pair.out_id) || used.contains(&pair.in_id) {
            return false;
        }
        used.insert(pair.out_id.clone());
        used.insert(pair.in_id.clone());
        true
    });
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transaction;

    fn day(d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2024, 6, d).unwrap()
    }

    /// 1 000 EUR leaving broker A on the 3rd and 998 EUR arriving at broker B on the 5th is one
    /// move minus a wire fee, not a withdrawal from the portfolio plus a fresh deposit into it.
    #[test]
    fn two_brokers_one_move_is_offered_as_a_pair() {
        let out = Transaction::cash("acc-a", TransactionKind::Withdrawal, day(3), dec!(1000), "EUR");
        let into = Transaction::cash("acc-b", TransactionKind::Deposit, day(5), dec!(998), "EUR");
        // Same amount, same day, but the account it left is the account it arrived on.
        let self_move = Transaction::cash("acc-a", TransactionKind::Deposit, day(3), dec!(1000), "EUR");

        let found = transfer_candidates(&[out.clone(), into.clone(), self_move]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].out_id, out.id);
        assert_eq!(found[0].in_id, into.id);
        assert_eq!(found[0].days_apart, 2);
    }

    /// A leg already linked to its partner is settled; a different currency is a conversion the
    /// amounts cannot be compared across; too far apart is two separate payments.
    #[test]
    fn nothing_is_offered_for_a_leg_that_is_already_answered() {
        let mut linked = Transaction::cash("acc-a", TransactionKind::Withdrawal, day(3), dec!(500), "EUR");
        linked.link_id = Some("pair-1".into());
        let arrival = Transaction::cash("acc-b", TransactionKind::Deposit, day(3), dec!(500), "EUR");
        assert!(transfer_candidates(&[linked, arrival.clone()]).is_empty());

        let out = Transaction::cash("acc-a", TransactionKind::Withdrawal, day(3), dec!(500), "USD");
        assert!(transfer_candidates(&[out.clone(), arrival.clone()]).is_empty());

        let late = Transaction::cash("acc-b", TransactionKind::Deposit, day(20), dec!(500), "EUR");
        let out = Transaction::cash("acc-a", TransactionKind::Withdrawal, day(3), dec!(500), "EUR");
        assert!(transfer_candidates(&[out, late]).is_empty());
    }

    /// One withdrawal cannot be the partner of two arrivals: the nearer one takes it and the
    /// other suggestion goes with it, so confirming the list never links a leg twice.
    #[test]
    fn a_leg_is_offered_to_one_partner_only() {
        let out = Transaction::cash("acc-a", TransactionKind::Withdrawal, day(3), dec!(700), "EUR");
        let near = Transaction::cash("acc-b", TransactionKind::Deposit, day(4), dec!(700), "EUR");
        let far = Transaction::cash("acc-c", TransactionKind::Deposit, day(7), dec!(700), "EUR");

        let found = transfer_candidates(&[out, far, near.clone()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].in_id, near.id);
    }
}
