//! Dividends the open positions are expected to pay, projected from what each instrument's source
//! reported it paid before. A forecast, never income: nothing here is written. See ADR-0056.

use super::dividends::median_gap;
use super::{DividendFrequency, Holdings, IncomeRecord};
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::DateRange;
use crate::model::{SecurityEvent, SecurityEventKind, TransactionKind};
use crate::money::{Currency, round_money};
use chrono::{Duration, Months, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A payment that is more than this many times the year's median is a special one, not a
/// schedule: projecting Costco's $15 special would forecast thirteen times its real income.
const SPECIAL_FACTOR: Decimal = Decimal::from_parts(3, 0, 0, false, 0);

/// Furthest a forecast looks ahead; beyond two years the year-ago pattern says nothing.
const MAX_HORIZON_DAYS: i64 = 731;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedDividend {
    pub security_id: String,
    /// The ex-date: holding the shares the day before it is what entitles the payment.
    pub ex_date: NaiveDate,
    /// When the cash should arrive — the ex-date plus the lag this portfolio's own payments of
    /// the instrument showed. `None` when none was ever recorded to measure it from.
    pub pay_date: Option<NaiveDate>,
    pub frequency: DividendFrequency,
    /// `true` when the source already reported this very payment and only its cash is still
    /// due; `false` for a date and an amount carried forward from the past year.
    pub reported: bool,
    /// Per share, in `currency`.
    #[serde(with = "rust_decimal::serde::str")]
    pub per_share: Decimal,
    pub currency: Currency,
    /// Shares held today; a sale or a purchase before the ex-date is not foreseen.
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Before withholding, at today's exchange rate.
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_base: Decimal,
    /// After the withholding share of the last payment received; `None` when none was received.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub net_base: Option<Decimal>,
}

impl ExpectedDividend {
    /// The day the money moves, which is what the window and the order are about.
    pub fn cash_date(&self) -> NaiveDate {
        self.pay_date.unwrap_or(self.ex_date)
    }
}

/// Every dividend expected with its cash date in `(range.from, range.to]`, soonest first.
///
/// `holdings` says how many shares (the lens's); `received` is the payment history the lag and
/// the withholding are read from (the whole portfolio's — see `positions_at`); `reported` is
/// every instrument event, of which only the provider's dividends are used.
pub fn expected_dividends(
    holdings: &Holdings,
    reported: &[SecurityEvent],
    received: &[IncomeRecord],
    range: DateRange,
    base: &str,
    rates: &impl RateLookup,
) -> Result<Vec<ExpectedDividend>> {
    let as_of = range.from;
    let to = range.to.min(as_of + Duration::days(MAX_HORIZON_DAYS));
    let mut out = Vec::new();

    for (security_id, position) in &holdings.positions {
        if position.quantity <= Decimal::ZERO {
            continue;
        }
        let history = reported_history(reported, security_id, as_of);
        let Some(schedule) = Schedule::read(&history, as_of) else {
            continue;
        };
        let own: Vec<&IncomeRecord> = received
            .iter()
            .filter(|r| r.kind == TransactionKind::Dividend && r.security_id.as_deref() == Some(security_id))
            .collect();
        let lag = pay_lag(&history, &own, schedule.gap);
        let net_share = own
            .iter()
            .filter(|r| r.gross_base > Decimal::ZERO)
            .max_by_key(|r| r.date)
            .map(|r| r.net_base / r.gross_base);
        let last_received = own.iter().map(|r| r.date).max();

        let mut taken: Vec<NaiveDate> = history.iter().map(|e| e.date).collect();
        // Newest first, so this year's payment claims next year's date before last year's could.
        for event in schedule.regular.iter().rev() {
            for years in 0..=2u32 {
                let ex_date = if years == 0 {
                    // The source's own record: due only while its cash has not arrived yet.
                    if last_received.is_some_and(|paid| paid >= event.date) {
                        continue;
                    }
                    event.date
                } else {
                    let Some(date) = event.date.checked_add_months(Months::new(12 * years)) else {
                        continue;
                    };
                    // A payment already reported near this date is the same one, not a second.
                    if taken
                        .iter()
                        .any(|t| (*t - date).num_days().abs() < schedule.gap / 2)
                    {
                        continue;
                    }
                    taken.push(date);
                    date
                };
                let pay_date = lag.map(|days| ex_date + Duration::days(days));
                let cash_date = pay_date.unwrap_or(ex_date);
                if cash_date <= as_of || cash_date > to {
                    continue;
                }
                let per_share = if years > 0 && schedule.frequency_carries_latest() {
                    schedule.latest
                } else {
                    event.amount
                };
                let rate = rate_to_base(rates, &event.currency, base, as_of)?;
                let gross_base = round_money(per_share * position.quantity * rate);
                out.push(ExpectedDividend {
                    security_id: security_id.clone(),
                    ex_date,
                    pay_date,
                    frequency: schedule.frequency,
                    reported: years == 0,
                    per_share,
                    currency: event.currency.clone(),
                    quantity: position.quantity,
                    gross_base,
                    net_base: net_share.map(|share| round_money(gross_base * share)),
                });
            }
        }
    }
    out.sort_by(|a, b| (a.cash_date(), &a.security_id).cmp(&(b.cash_date(), &b.security_id)));
    Ok(out)
}

struct Reported {
    date: NaiveDate,
    amount: Decimal,
    currency: Currency,
}

/// The provider's dividends of one instrument up to `as_of`, oldest first. The user's own notes
/// carry no amount and are not a record of payment.
fn reported_history(events: &[SecurityEvent], security_id: &str, as_of: NaiveDate) -> Vec<Reported> {
    let mut out: Vec<Reported> = events
        .iter()
        .filter(|e| e.security_id == security_id && e.kind == SecurityEventKind::Dividend)
        .filter(|e| e.source.is_some() && e.date <= as_of)
        .filter_map(|e| {
            Some(Reported {
                date: e.date,
                amount: e.amount.filter(|a| *a > Decimal::ZERO)?,
                currency: e.currency.clone()?,
            })
        })
        .collect();
    out.sort_by_key(|e| e.date);
    out
}

struct Schedule<'a> {
    frequency: DividendFrequency,
    gap: i64,
    /// The last year of payments without the specials, oldest first.
    regular: Vec<&'a Reported>,
    latest: Decimal,
}

impl<'a> Schedule<'a> {
    /// `None` for a payer with no schedule to project: fewer than two payments, gaps longer
    /// than a year, or silent for two of its own gaps — a suspended dividend is not a late one.
    fn read(history: &'a [Reported], as_of: NaiveDate) -> Option<Self> {
        let dates: BTreeSet<NaiveDate> = history.iter().map(|e| e.date).collect();
        let gap = median_gap(&dates)?;
        let frequency = DividendFrequency::of_gap(Some(gap));
        frequency.per_year()?;
        let last = *dates.last()?;
        if (as_of - last).num_days() > 2 * gap {
            return None;
        }

        let year: Vec<&Reported> = history
            .iter()
            .filter(|e| e.date > last - Duration::days(365))
            .collect();
        let mut amounts: Vec<Decimal> = year.iter().map(|e| e.amount).collect();
        amounts.sort();
        let median = amounts[amounts.len() / 2];
        let regular: Vec<&Reported> = year
            .into_iter()
            .filter(|e| amounts.len() == 1 || e.amount <= median * SPECIAL_FACTOR)
            .collect();
        let latest = regular.last()?.amount;
        Some(Schedule {
            frequency,
            gap,
            regular,
            latest,
        })
    }

    /// Monthly and quarterly payers pay equal amounts, so a raise shows in the latest one; a
    /// half-yearly or yearly payer splits interim and final, and only the same payment a year
    /// ago says what that one will be.
    fn frequency_carries_latest(&self) -> bool {
        matches!(
            self.frequency,
            DividendFrequency::Monthly | DividendFrequency::Quarterly
        )
    }
}

/// Median days from a reported ex-date to the payment this portfolio received for it. A payment
/// pairs with the latest ex-date before it, and only within one gap — an older one is another
/// payment's.
fn pay_lag(history: &[Reported], own: &[&IncomeRecord], gap: i64) -> Option<i64> {
    let mut lags: Vec<i64> = own
        .iter()
        .filter_map(|paid| {
            let ex = history.iter().rev().find(|e| e.date <= paid.date)?;
            let days = (paid.date - ex.date).num_days();
            (days <= gap).then_some(days)
        })
        .collect();
    if lags.is_empty() {
        return None;
    }
    lags.sort_unstable();
    Some(lags[lags.len() / 2])
}

/// A future date has no rate, so the forecast is at today's — the reading `plan_projection`
/// gives too.
fn rate_to_base(rates: &impl RateLookup, currency: &str, base: &str, as_of: NaiveDate) -> Result<Decimal> {
    if currency == base {
        return Ok(Decimal::ONE);
    }
    rates
        .rate_as_of(currency, base, as_of)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{currency}/{base}"),
            date: as_of,
        })
}
