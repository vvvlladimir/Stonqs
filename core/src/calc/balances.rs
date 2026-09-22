use crate::model::{Account, AccountKind, Transaction};
use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};

/// Maps each account to where its cash actually lands (a depot has none of its own).
pub fn settlement_accounts(accounts: &[Account]) -> HashMap<&str, &str> {
    accounts
        .iter()
        .map(|a| (a.id.as_str(), a.settlement_account_id()))
        .collect()
}

/// Cash balances per account and currency; preserving currencies avoids turning facts into
/// rate-dependent estimates, and zero-balance deposit accounts remain visible.
pub fn cash_balances(
    transactions: &[Transaction],
    accounts: &[Account],
    date: NaiveDate,
) -> BTreeMap<String, BTreeMap<Currency, Decimal>> {
    let settlement = settlement_accounts(accounts);
    let mut balances: BTreeMap<String, BTreeMap<Currency, Decimal>> = BTreeMap::new();

    for account in accounts {
        if account.kind == AccountKind::Deposit {
            balances
                .entry(account.id.clone())
                .or_default()
                .entry(account.currency.clone())
                .or_insert(Decimal::ZERO);
        }
    }

    for t in transactions {
        if t.date > date {
            continue;
        }
        let delta = t.cash_delta();
        // A charge billed in another currency is taken from that currency's balance.
        let legs = t.foreign_charge_legs();
        if delta.is_zero() && legs.is_empty() {
            continue;
        }
        // Account not in the list: skip rather than invent a balance nobody can see.
        let Some(target) = settlement.get(t.account_id.as_str()) else {
            continue;
        };
        let account = balances.entry((*target).to_string()).or_default();
        if !delta.is_zero() {
            *account.entry(t.currency.clone()).or_insert(Decimal::ZERO) += delta;
        }
        for (currency, amount) in legs {
            *account.entry(currency).or_insert(Decimal::ZERO) += amount;
        }
    }

    balances
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TransactionKind;
    use rust_decimal_macros::dec;

    fn date(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn buy_on_depot_spends_the_reference_account() {
        // Deposit 1000 EUR; 10 x 90 EUR + 1 EUR fee = 901 EUR, leaving 99 EUR cash.
        // Depot has no cash row.
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

        let balances = cash_balances(&[top_up, buy], &[cash.clone(), depot.clone()], date("2024-12-31"));

        assert_eq!(balances[&cash.id][&"EUR".to_string()], dec!(99));
        assert!(!balances.contains_key(&depot.id));
    }

    #[test]
    fn balance_is_as_of_the_date_and_per_currency() {
        // 500 EUR on Jan 10 and 200 USD on Jan 20, same account.
        // As of Jan 15: 500 EUR and no USD row yet.
        let cash = Account::deposit("Bank", "EUR");
        let eur = Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            date("2024-01-10"),
            dec!(500),
            "EUR",
        );
        let usd = Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            date("2024-01-20"),
            dec!(200),
            "USD",
        );

        let mid = cash_balances(
            std::slice::from_ref(&eur),
            std::slice::from_ref(&cash),
            date("2024-01-15"),
        );
        assert_eq!(mid[&cash.id][&"EUR".to_string()], dec!(500));

        let all = cash_balances(&[eur, usd], std::slice::from_ref(&cash), date("2024-12-31"));
        assert_eq!(all[&cash.id][&"EUR".to_string()], dec!(500));
        assert_eq!(all[&cash.id][&"USD".to_string()], dec!(200));
    }

    #[test]
    fn empty_deposit_account_is_listed() {
        let cash = Account::deposit("Bank", "EUR");
        let balances = cash_balances(&[], std::slice::from_ref(&cash), date("2024-01-01"));
        assert_eq!(balances[&cash.id][&"EUR".to_string()], Decimal::ZERO);
    }
}
