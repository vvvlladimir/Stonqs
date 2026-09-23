use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An amount the user means to have by a date, over the accounts they name. See ADR-0068.
///
/// It is an intention about the portfolio, not a reading of it, so it carries its own accounts
/// and ignores the picker — an empty `accounts` is the whole portfolio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub target_amount: Decimal,
    pub currency: Currency,
    /// `None` asks the other question: when would the stated monthly amount arrive.
    pub target_date: Option<NaiveDate>,
    /// Paid in every month from here on. `None` with a target date means "tell me what it takes".
    #[serde(with = "rust_decimal::serde::str_option")]
    pub monthly_amount: Option<Decimal>,
    /// The user's assumption, `0` being pure saving. Never this portfolio's measured return —
    /// the question is what it would take, not what happened (ADR-0059).
    #[serde(with = "rust_decimal::serde::str")]
    pub expected_return: Decimal,
    pub note: Option<String>,
    pub created_at: NaiveDate,
    /// The accounts that count towards it; empty is the whole portfolio.
    pub accounts: Vec<String>,
}

impl Goal {
    pub fn new(
        name: impl Into<String>,
        target_amount: Decimal,
        currency: &str,
        created_at: NaiveDate,
    ) -> Self {
        Goal {
            id: super::new_id(),
            name: name.into(),
            target_amount,
            currency: normalize_currency(currency),
            target_date: None,
            monthly_amount: None,
            expected_return: Decimal::ZERO,
            note: None,
            created_at,
            accounts: Vec::new(),
        }
    }

    /// A goal with nothing to reach has no progress to report.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("a goal needs a name".into()));
        }
        if self.target_amount <= Decimal::ZERO {
            return Err(Error::Invalid("a goal's target must be above zero".into()));
        }
        if self.monthly_amount.is_some_and(|m| m < Decimal::ZERO) {
            return Err(Error::Invalid("a monthly contribution cannot be negative".into()));
        }
        Ok(())
    }
}

/// What one account may take in one limit year: an ISA, a 401(k), an ИИС ceiling.
///
/// The app ships no country and no ceiling of its own — the rules differ per country *and* per
/// year, and a stale one stated confidently is worse than none (ADR-0068). It is measured, never
/// enforced: nothing here refuses a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionLimit {
    pub id: String,
    pub account_id: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: Currency,
    /// `MM-DD`: the day the limit year opens. The UK's begins on 6 April, so this is not
    /// always `01-01`.
    pub year_starts_on: String,
    pub note: Option<String>,
}

impl ContributionLimit {
    pub fn new(account_id: &str, name: impl Into<String>, amount: Decimal, currency: &str) -> Self {
        ContributionLimit {
            id: super::new_id(),
            account_id: account_id.to_string(),
            name: name.into(),
            amount,
            currency: normalize_currency(currency),
            year_starts_on: "01-01".to_string(),
            note: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("a limit needs a name".into()));
        }
        if self.amount <= Decimal::ZERO {
            return Err(Error::Invalid("a limit must be above zero".into()));
        }
        self.month_day()?;
        Ok(())
    }

    /// The `(month, day)` the limit year opens on.
    pub fn month_day(&self) -> Result<(u32, u32)> {
        let (month, day) = self
            .year_starts_on
            .split_once('-')
            .ok_or_else(|| Error::Invalid(format!("{:?} is not MM-DD", self.year_starts_on)))?;
        let parsed = (
            month.parse::<u32>().ok().filter(|m| (1..=12).contains(m)),
            day.parse::<u32>().ok().filter(|d| (1..=31).contains(d)),
        );
        match parsed {
            (Some(month), Some(day)) => Ok((month, day)),
            _ => Err(Error::Invalid(format!("{:?} is not MM-DD", self.year_starts_on))),
        }
    }

    /// The limit year `date` falls in: `[from, to]`, both inclusive.
    ///
    /// A year opening on 6 April runs to 5 April, so a deposit on 5 April 2025 belongs to the
    /// year that opened in 2024 — reading it into the calendar year would spend the wrong
    /// allowance.
    pub fn year_of(&self, date: NaiveDate) -> Result<(NaiveDate, NaiveDate)> {
        use chrono::Datelike;
        let (month, day) = self.month_day()?;
        let start_in = |year: i32| {
            NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| Error::Invalid(format!("no {month}-{day} in {year}")))
        };
        let this_year = start_in(date.year())?;
        let from = if date >= this_year {
            this_year
        } else {
            start_in(date.year() - 1)?
        };
        let next = start_in(from.year() + 1)?;
        Ok((from, next.pred_opt().unwrap_or(next)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// A UK allowance year runs 6 April to 5 April, so the two days either side of it sit in
    /// different years even though they share a month.
    #[test]
    fn a_limit_year_can_open_on_any_day() {
        let mut limit = ContributionLimit::new("acc", "ISA", dec!(20000), "GBP");
        limit.year_starts_on = "04-06".into();

        assert_eq!(
            limit.year_of(d(2025, 4, 6)).unwrap(),
            (d(2025, 4, 6), d(2026, 4, 5))
        );
        assert_eq!(
            limit.year_of(d(2025, 4, 5)).unwrap(),
            (d(2024, 4, 6), d(2025, 4, 5))
        );
        assert_eq!(
            limit.year_of(d(2025, 12, 31)).unwrap(),
            (d(2025, 4, 6), d(2026, 4, 5))
        );
    }

    #[test]
    fn a_calendar_limit_year_is_the_calendar_year() {
        let limit = ContributionLimit::new("acc", "ИИС", dec!(1000000), "RUB");
        assert_eq!(
            limit.year_of(d(2025, 7, 1)).unwrap(),
            (d(2025, 1, 1), d(2025, 12, 31))
        );
    }

    #[test]
    fn a_broken_month_day_is_refused() {
        let mut limit = ContributionLimit::new("acc", "ISA", dec!(1), "GBP");
        limit.year_starts_on = "13-40".into();
        assert!(limit.validate().is_err());
    }
}
