//! The payments grid: every line of the cash story against a time axis.
//!
//! One rollup rather than seven, because the question it answers is comparative — "what did
//! June bring, and how does it sit against what I paid in and what I sold". Every figure here
//! already exists somewhere in [`Holdings`]; the grid only lays them on one axis, so a caller
//! never adds two reports together to get a third.

use super::Holdings;
use crate::model::TransactionKind;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Width of one column of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentPeriod {
    Month,
    Quarter,
    Year,
}

/// One column: an interval, not a label. The name of a month belongs to the frontend,
/// where the language is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentBucket {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub year: i32,
    /// 1..=12 for a month, 1..=4 for a quarter, always 1 for a year.
    pub index: u32,
}

/// One row of the grid. The income kinds are named one by one because a dividend and a
/// charged interest are read differently; everything else the portfolio does is one line each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentLine {
    Dividends,
    Interest,
    /// Interest the broker charged: already negative, so the subtotal needs no sign rule.
    InterestCharge,
    /// Income that is neither: a crypto reward, a card cashback.
    OtherIncome,
    Fees,
    Taxes,
    /// Money and securities crossing the portfolio boundary — paid in minus taken out.
    Savings,
    /// The result of disposals settled in the bucket, not the money they returned.
    ClosedTrades,
}

impl PaymentLine {
    /// Whether the line is part of what the portfolio *earned*. Fees and taxes are not: they
    /// are what it cost. Savings are not earnings at all — that is the whole point of the row.
    pub fn is_earning(self) -> bool {
        matches!(
            self,
            PaymentLine::Dividends
                | PaymentLine::Interest
                | PaymentLine::InterestCharge
                | PaymentLine::OtherIncome
        )
    }

    fn of_income(kind: TransactionKind) -> Self {
        match kind {
            TransactionKind::Dividend => PaymentLine::Dividends,
            TransactionKind::Interest => PaymentLine::Interest,
            TransactionKind::InterestCharge => PaymentLine::InterestCharge,
            _ => PaymentLine::OtherIncome,
        }
    }
}

/// One row: an amount per bucket, in the same order as [`PaymentGrid::buckets`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentRow {
    pub line: PaymentLine,
    pub amounts: Vec<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
}

/// The same axis, split by payer instead of by kind. Only income: a fee has no instrument
/// that "paid" it, and a disposal is a trade, which the trades screen already lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPaymentRow {
    /// `None` is the account itself — interest owes nothing to an instrument.
    pub security_id: Option<String>,
    pub amounts: Vec<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
}

/// Every line of the grid over one continuous axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentGrid {
    pub period: PaymentPeriod,
    pub buckets: Vec<PaymentBucket>,
    /// Kind rows, in the declaration order of [`PaymentLine`], empty ones dropped.
    pub lines: Vec<PaymentRow>,
    /// Payer rows, largest total first.
    pub securities: Vec<SecurityPaymentRow>,
    /// Sum of the earning lines per bucket — what the portfolio made, before what it cost.
    pub earnings: Vec<Decimal>,
    /// Running sum of `earnings`: the line that only ever turns down in a bad month.
    pub cumulative: Vec<Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub earnings_total: Decimal,
}

/// Lays every dated figure of `holdings` on one axis over `[from, to]`.
///
/// Empty buckets are kept: a year with no dividend in August is a fact about the payer, and a
/// grid that silently skips the column would hide it.
pub fn payment_grid(
    holdings: &Holdings,
    from: NaiveDate,
    to: NaiveDate,
    period: PaymentPeriod,
) -> PaymentGrid {
    let buckets = buckets_between(from, to, period);
    let width = buckets.len();
    let slot = |date: NaiveDate| index_of(&buckets, date);

    let mut lines: Vec<(PaymentLine, Vec<Decimal>)> = Vec::new();
    let mut add = |line: PaymentLine, at: usize, amount: Decimal| {
        let row = match lines.iter_mut().find(|(l, _)| *l == line) {
            Some(row) => row,
            None => {
                lines.push((line, vec![Decimal::ZERO; width]));
                lines.last_mut().expect("just pushed")
            }
        };
        row.1[at] += amount;
    };

    let mut payers: Vec<(Option<String>, Vec<Decimal>)> = Vec::new();
    for record in &holdings.income {
        let Some(at) = slot(record.date) else { continue };
        add(PaymentLine::of_income(record.kind), at, record.net_base);
        match payers.iter_mut().find(|(id, _)| *id == record.security_id) {
            Some(payer) => payer.1[at] += record.net_base,
            None => {
                let mut amounts = vec![Decimal::ZERO; width];
                amounts[at] = record.net_base;
                payers.push((record.security_id.clone(), amounts));
            }
        }
    }
    for charge in &holdings.charges {
        let Some(at) = slot(charge.date) else { continue };
        let line = match charge.kind {
            TransactionKind::Fee | TransactionKind::FeeRefund => PaymentLine::Fees,
            _ => PaymentLine::Taxes,
        };
        add(line, at, charge.amount_base);
    }
    for flow in &holdings.external_flows {
        if let Some(at) = slot(flow.date) {
            add(PaymentLine::Savings, at, flow.amount_base);
        }
    }
    for gain in &holdings.realized {
        if let Some(at) = slot(gain.date) {
            add(PaymentLine::ClosedTrades, at, gain.gain_base);
        }
    }

    lines.sort_by_key(|(line, _)| *line);
    let lines: Vec<PaymentRow> = lines
        .into_iter()
        .map(|(line, amounts)| PaymentRow {
            total: amounts.iter().sum(),
            line,
            amounts,
        })
        .collect();

    let mut securities: Vec<SecurityPaymentRow> = payers
        .into_iter()
        .map(|(security_id, amounts)| SecurityPaymentRow {
            total: amounts.iter().sum(),
            security_id,
            amounts,
        })
        .collect();
    // Largest payer first; ties keep a stable order so the table does not shuffle itself.
    securities.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then_with(|| a.security_id.cmp(&b.security_id))
    });

    let earnings: Vec<Decimal> = (0..width)
        .map(|at| {
            lines
                .iter()
                .filter(|row| row.line.is_earning())
                .map(|row| row.amounts[at])
                .sum()
        })
        .collect();
    let mut running = Decimal::ZERO;
    let cumulative: Vec<Decimal> = earnings
        .iter()
        .map(|amount| {
            running += amount;
            running
        })
        .collect();

    PaymentGrid {
        period,
        buckets,
        lines,
        securities,
        earnings_total: running,
        earnings,
        cumulative,
    }
}

/// Continuous columns covering `[from, to]`; the first and last are clipped to the range, so
/// a window starting mid-month does not claim the days before it.
fn buckets_between(from: NaiveDate, to: NaiveDate, period: PaymentPeriod) -> Vec<PaymentBucket> {
    let mut out = Vec::new();
    if to < from {
        return out;
    }
    let mut cursor = start_of(from, period);
    while cursor <= to {
        let next = next_start(cursor, period);
        out.push(PaymentBucket {
            from: cursor.max(from),
            to: (next.pred_opt().unwrap_or(next)).min(to),
            year: cursor.year(),
            index: match period {
                PaymentPeriod::Month => cursor.month(),
                PaymentPeriod::Quarter => (cursor.month() - 1) / 3 + 1,
                PaymentPeriod::Year => 1,
            },
        });
        cursor = next;
    }
    out
}

fn start_of(date: NaiveDate, period: PaymentPeriod) -> NaiveDate {
    let month = match period {
        PaymentPeriod::Month => date.month(),
        PaymentPeriod::Quarter => (date.month() - 1) / 3 * 3 + 1,
        PaymentPeriod::Year => 1,
    };
    NaiveDate::from_ymd_opt(date.year(), month, 1).expect("the first of a month always exists")
}

fn next_start(start: NaiveDate, period: PaymentPeriod) -> NaiveDate {
    let months = match period {
        PaymentPeriod::Month => 1,
        PaymentPeriod::Quarter => 3,
        PaymentPeriod::Year => 12,
    };
    let total = start.month0() + months;
    NaiveDate::from_ymd_opt(start.year() + (total / 12) as i32, total % 12 + 1, 1)
        .expect("the first of a month always exists")
}

/// Which column a date falls in. Buckets are contiguous, so a linear scan over a handful of
/// columns beats date arithmetic that has to know about the clipped ends.
fn index_of(buckets: &[PaymentBucket], date: NaiveDate) -> Option<usize> {
    buckets
        .iter()
        .position(|bucket| date >= bucket.from && date <= bucket.to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// A year of months is twelve columns, the first clipped to the window's own start.
    #[test]
    fn months_are_continuous_and_clipped_to_the_window() {
        let buckets = buckets_between(day(2024, 1, 15), day(2024, 3, 10), PaymentPeriod::Month);

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].from, day(2024, 1, 15));
        assert_eq!(buckets[0].to, day(2024, 1, 31));
        assert_eq!(buckets[1].from, day(2024, 2, 1));
        // 2024 is a leap year: February ends on the 29th.
        assert_eq!(buckets[1].to, day(2024, 2, 29));
        assert_eq!(buckets[2].to, day(2024, 3, 10));
        assert_eq!(buckets[2].index, 3);
    }

    /// Quarters start in January, April, July, October whatever day the window opens on.
    #[test]
    fn quarters_start_where_a_quarter_starts() {
        let buckets = buckets_between(day(2024, 2, 1), day(2024, 7, 1), PaymentPeriod::Quarter);

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].index, 1);
        assert_eq!(buckets[1].from, day(2024, 4, 1));
        assert_eq!(buckets[1].index, 2);
        assert_eq!(buckets[2].from, day(2024, 7, 1));
        assert_eq!(buckets[2].to, day(2024, 7, 1));
    }

    /// A December bucket rolls the year over rather than producing month 13.
    #[test]
    fn the_axis_crosses_a_year_boundary() {
        let buckets = buckets_between(day(2023, 12, 1), day(2024, 1, 31), PaymentPeriod::Month);

        assert_eq!(buckets.len(), 2);
        assert_eq!((buckets[0].year, buckets[0].index), (2023, 12));
        assert_eq!((buckets[1].year, buckets[1].index), (2024, 1));
    }
}
