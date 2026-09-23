use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::Transaction;
use chrono::Datelike;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Money moved through the portfolio in one calendar month; separate year/month fields
/// keep the wire representation straightforward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthlyNet {
    pub year: i32,
    pub month: u32,
    pub count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

/// Net monthly journal in base currency using signed [`Transaction::cash_delta`], with
/// each transaction converted at its own date and rate.
pub fn transactions_net_by_month(
    transactions: &[Transaction],
    base: &str,
    rates: &dyn RateLookup,
) -> Result<Vec<MonthlyNet>> {
    let mut out: Vec<MonthlyNet> = Vec::new();
    // No map: transactions arrive sorted by date, so checking only the last group suffices.
    let mut sorted: Vec<&Transaction> = transactions.iter().collect();
    sorted.sort_by_key(|t| t.date);

    for t in sorted {
        let net = transaction_net_base(t, base, rates)?;
        let (year, month) = (t.date.year(), t.date.month());
        match out.last_mut() {
            Some(last) if last.year == year && last.month == month => {
                last.count += 1;
                last.net_base += net;
            }
            _ => out.push(MonthlyNet {
                year,
                month,
                count: 1,
                net_base: net,
            }),
        }
    }
    Ok(out)
}

/// Signed cash side of one transaction, in base currency.
pub fn transaction_net_base(t: &Transaction, base: &str, rates: &dyn RateLookup) -> Result<Decimal> {
    let mut net = t.cash_delta() * super::resolve_rate(t, base, rates)?;
    // A charge billed in another currency left that balance; it is converted at its own rate.
    for (currency, amount) in t.foreign_charge_legs() {
        net += amount * super::holdings::charge_rate(t, &currency, base, rates)?;
    }
    Ok(net)
}

/// Unsigned `amount` in base currency — a separate column from [`transaction_net_base`]
/// (a buy's amount is positive, its net cash effect is negative).
pub fn transaction_amount_base(t: &Transaction, base: &str, rates: &dyn RateLookup) -> Result<Decimal> {
    Ok(t.amount * super::resolve_rate(t, base, rates)?)
}

/// Money that moved through the portfolio in one calendar year. A separate type rather
/// than `MonthlyNet` with `month: 0` — the "month zero" convention isn't self-explanatory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YearlyNet {
    pub year: i32,
    pub count: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

/// Net journal by year — rolled up from already-computed [`MonthlyNet`] rather than
/// re-walking transactions, so there's no second chance to diverge from the monthly figure.
pub fn transactions_net_by_year(months: &[MonthlyNet]) -> Vec<YearlyNet> {
    let mut out: Vec<YearlyNet> = Vec::new();
    for m in months {
        match out.last_mut() {
            Some(last) if last.year == m.year => {
                last.count += m.count;
                last.net_base += m.net_base;
            }
            _ => out.push(YearlyNet {
                year: m.year,
                count: m.count,
                net_base: m.net_base,
            }),
        }
    }
    out
}
