//! What was paid into one account against the ceiling it is allowed — an ISA, a 401(k), an ИИС.
//!
//! A contribution is money that entered the *portfolio*: a deposit, and a transfer leg whose
//! partner is not in the ledger. Moving money between two of the user's own accounts is not a
//! contribution and never eats an allowance (ADR-0068). What left the account is netted off only
//! when the limit says a withdrawal gives allowance back (ADR-0071). It is measured, never
//! enforced: nothing here refuses anything.

use super::holdings::paired_links;
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{ContributionLimit, Transaction, TransactionKind};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One limit year of one account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitUsage {
    pub limit_id: String,
    pub account_id: String,
    pub name: String,
    /// The limit year this reading covers, both ends inclusive.
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub allowance: Decimal,
    /// Paid in over that year — net of withdrawals only when they restore allowance — never
    /// below zero.
    #[serde(with = "rust_decimal::serde::str")]
    pub used: Decimal,
    /// What is left of the allowance; zero once it is spent.
    #[serde(with = "rust_decimal::serde::str")]
    pub remaining: Decimal,
    /// `used / allowance`; above one when the ceiling was passed.
    #[serde(with = "rust_decimal::serde::str")]
    pub share: Decimal,
    pub currency: String,
    /// Echoes the limit's switch, so an edit form starts from what is stored.
    pub withdrawals_restore: bool,
}

/// Reads `limit` over the limit year that `as_of` falls in.
///
/// Amounts are converted into the limit's own currency at the rate of the day each one moved —
/// an allowance is stated in one currency and spent in whatever the deposit arrived in.
pub fn limit_usage(
    limit: &ContributionLimit,
    transactions: &[Transaction],
    as_of: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<LimitUsage> {
    limit.validate()?;
    let (from, to) = limit.year_of(as_of)?;
    let used = contributions_between(
        transactions,
        &limit.account_id,
        from,
        to,
        &limit.currency,
        limit.withdrawals_restore,
        rates,
    )?;
    // A year whose withdrawals exceed its deposits has spent no allowance; it has not earned one.
    let used = used.max(Decimal::ZERO);

    Ok(LimitUsage {
        limit_id: limit.id.clone(),
        account_id: limit.account_id.clone(),
        name: limit.name.clone(),
        from,
        to,
        allowance: limit.amount,
        remaining: (limit.amount - used).max(Decimal::ZERO),
        share: if limit.amount.is_zero() {
            Decimal::ZERO
        } else {
            used / limit.amount
        },
        used,
        currency: limit.currency.clone(),
        withdrawals_restore: limit.withdrawals_restore,
    })
}

/// Money that entered the portfolio through one account in `[from, to]`; with `net`, less what
/// left it the same way.
///
/// The paired-link reading is the ledger's, not this function's invention: a transfer with a
/// partner in the same set moved money inside the portfolio, and counting it would make one
/// move between two own accounts look like a fresh year's allowance spent.
pub fn contributions_between(
    transactions: &[Transaction],
    account_id: &str,
    from: NaiveDate,
    to: NaiveDate,
    currency: &str,
    net: bool,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    let paired = paired_links(transactions);
    let mut total = Decimal::ZERO;

    for t in transactions {
        if t.account_id != account_id || t.date < from || t.date > to {
            continue;
        }
        let signed = match t.kind {
            TransactionKind::Deposit => t.amount,
            TransactionKind::Withdrawal => -t.amount,
            TransactionKind::TransferIn | TransactionKind::TransferOut => {
                if t.link_id.as_deref().is_some_and(|link| paired.contains(link)) {
                    continue;
                }
                t.cash_delta()
            }
            _ => continue,
        };
        if !net && signed < Decimal::ZERO {
            continue;
        }
        total += rates.convert(signed, &t.currency, currency, t.date)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const ACC: &str = "acc-cash";
    const OTHER: &str = "acc-other";

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    struct Same;
    impl RateLookup for Same {
        fn rate_as_of(&self, from: &str, to: &str, _date: NaiveDate) -> Result<Option<Decimal>> {
            Ok((from == to).then_some(Decimal::ONE))
        }
    }

    fn limit() -> ContributionLimit {
        ContributionLimit::new(ACC, "ISA", dec!(20000), "GBP")
    }

    /// 12 000 paid in over the year against a 20 000 ceiling: 8 000 left, 60% spent.
    #[test]
    fn deposits_spend_the_allowance() {
        let txs = vec![
            Transaction::cash(ACC, TransactionKind::Deposit, d(2025, 2, 1), dec!(8000), "GBP"),
            Transaction::cash(ACC, TransactionKind::Deposit, d(2025, 9, 1), dec!(4000), "GBP"),
            // Another account's deposit is another allowance's business.
            Transaction::cash(OTHER, TransactionKind::Deposit, d(2025, 3, 1), dec!(5000), "GBP"),
            // Last year's, whatever the ceiling is today.
            Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 5, 1), dec!(9000), "GBP"),
        ];
        let usage = limit_usage(&limit(), &txs, d(2025, 10, 1), &Same).unwrap();

        assert_eq!(usage.from, d(2025, 1, 1));
        assert_eq!(usage.to, d(2025, 12, 31));
        assert_eq!(usage.used, dec!(12000));
        assert_eq!(usage.remaining, dec!(8000));
        assert_eq!(usage.share, dec!(0.6));
    }

    /// Two legs of one internal move are not a contribution; a leg with no partner is.
    #[test]
    fn an_internal_move_eats_no_allowance() {
        let mut out = Transaction::cash(
            OTHER,
            TransactionKind::TransferOut,
            d(2025, 2, 1),
            dec!(5000),
            "GBP",
        );
        out.link_id = Some("move-1".into());
        let mut into = Transaction::cash(ACC, TransactionKind::TransferIn, d(2025, 2, 1), dec!(5000), "GBP");
        into.link_id = Some("move-1".into());
        // A leg whose partner was never imported is money that crossed the boundary.
        let lonely = Transaction::cash(ACC, TransactionKind::TransferIn, d(2025, 3, 1), dec!(3000), "GBP");

        let usage = limit_usage(&limit(), &[out, into, lonely], d(2025, 6, 1), &Same).unwrap();
        assert_eq!(usage.used, dec!(3000));
    }

    /// 5 000 in, 9 000 out. By default a withdrawal gives nothing back: 5 000 used, 15 000 left.
    /// A flexible allowance nets it off: 5 000 - 9 000 = -4 000, read as 0 used, never as credit.
    #[test]
    fn withdrawals_count_only_when_they_restore_allowance() {
        let txs = vec![
            Transaction::cash(ACC, TransactionKind::Deposit, d(2025, 2, 1), dec!(5000), "GBP"),
            Transaction::cash(ACC, TransactionKind::Withdrawal, d(2025, 4, 1), dec!(9000), "GBP"),
        ];
        let usage = limit_usage(&limit(), &txs, d(2025, 6, 1), &Same).unwrap();
        assert_eq!(usage.used, dec!(5000));
        assert_eq!(usage.remaining, dec!(15000));

        let mut flexible = limit();
        flexible.withdrawals_restore = true;
        let usage = limit_usage(&flexible, &txs, d(2025, 6, 1), &Same).unwrap();
        assert_eq!(usage.used, Decimal::ZERO);
        assert_eq!(usage.remaining, dec!(20000));
    }

    /// A UK year opens on 6 April, so a deposit on the 5th belongs to the year before.
    #[test]
    fn the_limit_year_is_the_limits_own() {
        let mut limit = limit();
        limit.year_starts_on = "04-06".into();
        let txs = vec![
            Transaction::cash(ACC, TransactionKind::Deposit, d(2025, 4, 5), dec!(7000), "GBP"),
            Transaction::cash(ACC, TransactionKind::Deposit, d(2025, 4, 6), dec!(2000), "GBP"),
        ];

        let new_year = limit_usage(&limit, &txs, d(2025, 5, 1), &Same).unwrap();
        assert_eq!(new_year.from, d(2025, 4, 6));
        assert_eq!(new_year.used, dec!(2000));

        let old_year = limit_usage(&limit, &txs, d(2025, 4, 1), &Same).unwrap();
        assert_eq!(old_year.used, dec!(7000));
    }
}
