//! Account scopes rewrite linked transaction legs into external flows.
//! This keeps depot-only and cash-only views economically consistent.

use crate::model::{Account, Transaction, TransactionKind};
use std::collections::{HashMap, HashSet};

/// Rewrites all portfolio transactions for the selected account scope.
/// Both linked legs are inspected before one is converted to an external flow.
pub fn scoped_transactions(
    transactions: &[Transaction],
    accounts: &[Account],
    scope: &[String],
) -> Vec<Transaction> {
    let included: HashSet<&str> = scope.iter().map(String::as_str).collect();

    // Avoid rewriting the common full-portfolio case; scopes may contain duplicates.
    if accounts.iter().all(|a| included.contains(a.id.as_str())) {
        return transactions.to_vec();
    }

    let settlement: HashMap<&str, &str> = accounts
        .iter()
        .map(|a| (a.id.as_str(), a.settlement_account_id()))
        .collect();

    let mut sides: HashMap<&str, Vec<&Transaction>> = HashMap::new();
    for t in transactions {
        if let Some(link) = t.link_id.as_deref() {
            sides.entry(link).or_default().push(t);
        }
    }

    // Unknown accounts are treated as their own cash leg and remain out of scope.
    let cash_leg = |account_id: &str| -> String {
        settlement
            .get(account_id)
            .copied()
            .unwrap_or(account_id)
            .to_string()
    };

    let peer_in_scope = |t: &Transaction, use_cash_leg: bool| -> bool {
        let Some(link) = t.link_id.as_deref() else {
            return false;
        };
        let Some(peers) = sides.get(link) else {
            return false;
        };
        peers.iter().any(|p| {
            if p.id == t.id {
                return false;
            }
            let leg = if use_cash_leg {
                cash_leg(&p.account_id)
            } else {
                p.account_id.clone()
            };
            included.contains(leg.as_str())
        })
    };

    let mut out = Vec::with_capacity(transactions.len());
    for t in transactions {
        let securities_side = included.contains(t.account_id.as_str());
        let money_side = included.contains(cash_leg(&t.account_id).as_str());

        match t.kind {
            // A trade uses the depot for securities and its settlement account for cash.
            TransactionKind::Buy | TransactionKind::Sell => match (securities_side, money_side) {
                (true, true) => out.push(t.clone()),
                (true, false) => out.push(as_delivery(t)),
                (false, true) => out.push(as_external_cash(t)),
                (false, false) => {}
            },

            // Security deliveries affect only the depot side.
            TransactionKind::DeliveryInbound | TransactionKind::DeliveryOutbound => {
                if securities_side {
                    out.push(t.clone());
                }
            }

            // An out-of-scope transfer leg becomes an external delivery.
            TransactionKind::SecurityTransferIn | TransactionKind::SecurityTransferOut => {
                if !securities_side {
                    continue;
                }
                if peer_in_scope(t, false) {
                    out.push(t.clone());
                } else {
                    out.push(as_delivery(t));
                }
            }

            // Cash transfers use the same in-scope/out-of-scope rule.
            TransactionKind::TransferIn | TransactionKind::TransferOut => {
                if !money_side {
                    continue;
                }
                if peer_in_scope(t, true) {
                    out.push(t.clone());
                } else {
                    out.push(as_external_cash(t));
                }
            }

            // Income, charges, deposits, and withdrawals belong to the cash leg.
            _ => {
                if money_side {
                    out.push(t.clone());
                }
            }
        }
    }
    out
}

/// Converts a depot leg whose cash counterpart is outside the scope.
/// Keeping fees and taxes preserves the original cost-basis calculation.
fn as_delivery(t: &Transaction) -> Transaction {
    let mut out = t.clone();
    out.kind = match t.kind {
        TransactionKind::Buy | TransactionKind::SecurityTransferIn => TransactionKind::DeliveryInbound,
        _ => TransactionKind::DeliveryOutbound,
    };
    // The counterpart is absent, so retaining the link would confuse in-transit lots.
    out.link_id = None;
    // What it was is not derivable from what it became: a delivery carries no commission, and
    // this one does. `trading_volume` reads it; the cash-side rewrite deliberately sets nothing,
    // because from a deposit account the trade happened somewhere else.
    out.scoped_from = Some(t.kind);
    out
}

/// Converts a cash leg whose securities counterpart is outside the scope.
/// Fees and taxes stay in `amount` because deposits and withdrawals do not use gross trade value.
fn as_external_cash(t: &Transaction) -> Transaction {
    let amount = t.gross_in_transaction_currency();
    let mut out = t.clone();
    out.kind = match t.kind {
        TransactionKind::Buy | TransactionKind::TransferOut => TransactionKind::Withdrawal,
        _ => TransactionKind::Deposit,
    };
    out.amount = amount;
    // Only what `amount` just absorbed is cleared: a charge billed in another currency is not
    // in this total and would be money the lens made disappear.
    if out.fee_currency.is_none() {
        out.fees = rust_decimal::Decimal::ZERO;
    }
    if out.tax_currency.is_none() {
        out.taxes = rust_decimal::Decimal::ZERO;
    }
    out.security_id = None;
    out.quantity = rust_decimal::Decimal::ZERO;
    out.price = rust_decimal::Decimal::ZERO;
    out.link_id = None;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Account;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    /// Deposit 1000, then buy 10 x 90 plus a 1-unit fee.
    fn fixture() -> (Account, Account, Vec<Transaction>) {
        let cash = Account::deposit("Bank", "EUR");
        let depot = Account::securities("Broker", "EUR", &cash.id);
        let top_up = Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            date("2024-01-02"),
            dec!(1000),
            "EUR",
        );
        let buy = Transaction::buy(&depot.id, "sec", date("2024-01-03"), dec!(10), dec!(90), "EUR")
            .with_fees(dec!(1));
        let dividend = Transaction::dividend(&cash.id, "sec", date("2024-03-01"), dec!(5), "EUR");
        (cash, depot, vec![top_up, buy, dividend])
    }

    #[test]
    fn whole_portfolio_is_untouched() {
        let (cash, depot, tx) = fixture();
        let scope = vec![cash.id.clone(), depot.id.clone()];
        assert_eq!(scoped_transactions(&tx, &[cash, depot], &scope), tx);
    }

    /// Depot-only scope sees the purchase as an external delivery; cash stays out of scope.
    #[test]
    fn a_depot_alone_sees_purchases_as_inbound_deliveries() {
        let (cash, depot, tx) = fixture();
        let out = scoped_transactions(&tx, &[cash, depot.clone()], std::slice::from_ref(&depot.id));

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, TransactionKind::DeliveryInbound);
        assert_eq!(out[0].quantity, dec!(10));
        // Cost basis is unchanged: 10 x 90 + 1 = 901.
        assert_eq!(out[0].gross_in_transaction_currency(), dec!(901));
        assert_eq!(out[0].cash_delta(), Decimal::ZERO);
    }

    /// Cash-only scope sees the purchase as money leaving the selected perimeter.
    #[test]
    fn a_deposit_account_alone_sees_purchases_as_withdrawals() {
        let (cash, depot, tx) = fixture();
        let out = scoped_transactions(&tx, &[cash.clone(), depot], std::slice::from_ref(&cash.id));

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].kind, TransactionKind::Deposit);
        assert_eq!(out[1].kind, TransactionKind::Withdrawal);
        assert_eq!(out[1].amount, dec!(901));
        assert_eq!(out[1].cash_delta(), dec!(-901));
        assert!(out[1].security_id.is_none());
        assert_eq!(out[2].kind, TransactionKind::Dividend);

        // Statement balance: 1000 - 901 + 5 = 104.
        let balance: Decimal = out.iter().map(|t| t.cash_delta()).sum();
        assert_eq!(balance, dec!(104));
    }

    /// An in-scope transfer stays internal; crossing the scope boundary makes it external.
    #[test]
    fn a_transfer_is_external_only_when_it_crosses_the_boundary() {
        let a = Account::deposit("A", "EUR");
        let b = Account::deposit("B", "EUR");
        let (out_side, in_side) =
            Transaction::cash_transfer(&a.id, &b.id, date("2024-02-01"), dec!(100), "EUR");
        let tx = vec![out_side, in_side];
        let accounts = [a.clone(), b.clone()];

        let both = scoped_transactions(&tx, &accounts, &[a.id.clone(), b.id.clone()]);
        assert_eq!(both[0].kind, TransactionKind::TransferOut);
        assert_eq!(both[1].kind, TransactionKind::TransferIn);

        let only_a = scoped_transactions(&tx, &accounts, std::slice::from_ref(&a.id));
        assert_eq!(only_a.len(), 1);
        assert_eq!(only_a[0].kind, TransactionKind::Withdrawal);

        let only_b = scoped_transactions(&tx, &accounts, std::slice::from_ref(&b.id));
        assert_eq!(only_b.len(), 1);
        assert_eq!(only_b[0].kind, TransactionKind::Deposit);
    }

    /// A transfer to an out-of-scope depot must not leave an in-transit lot.
    #[test]
    fn a_security_transfer_across_the_boundary_becomes_a_delivery() {
        let cash = Account::deposit("Bank", "EUR");
        let from = Account::securities("From", "EUR", &cash.id);
        let to = Account::securities("To", "EUR", &cash.id);
        let (out_side, in_side) =
            Transaction::security_transfer(&from.id, &to.id, "sec", date("2024-05-01"), dec!(3), "EUR");
        let tx = vec![out_side, in_side];
        let accounts = [cash, from.clone(), to.clone()];

        let only_to = scoped_transactions(&tx, &accounts, std::slice::from_ref(&to.id));
        assert_eq!(only_to.len(), 1);
        assert_eq!(only_to[0].kind, TransactionKind::DeliveryInbound);
        assert!(only_to[0].link_id.is_none());

        let only_from = scoped_transactions(&tx, &accounts, std::slice::from_ref(&from.id));
        assert_eq!(only_from.len(), 1);
        assert_eq!(only_from[0].kind, TransactionKind::DeliveryOutbound);
    }
}
