use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// How often a plan fires. Deliberately not [`crate::calc::Period`]: that one is a reporting
/// axis that splits a range for a chart, this one is a calendar recurrence. See ADR-0033.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Interval {
    Week,
    /// Quarterly is `count = 3`, semi-annual `6`, annual `12`.
    Month,
}

impl Interval {
    pub fn as_str(self) -> &'static str {
        match self {
            Interval::Week => "WEEK",
            Interval::Month => "MONTH",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "WEEK" => Ok(Interval::Week),
            "MONTH" => Ok(Interval::Month),
            other => Err(Error::Invalid(format!("unknown plan interval {other}"))),
        }
    }
}

/// When a plan fires: every `count` units of `unit`, counted from `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub start: NaiveDate,
    /// `None` means the plan runs until it is stopped.
    #[serde(default)]
    pub end: Option<NaiveDate>,
    pub unit: Interval,
    pub count: u32,
}

impl Schedule {
    pub fn monthly(start: NaiveDate) -> Self {
        Schedule {
            start,
            end: None,
            unit: Interval::Month,
            count: 1,
        }
    }

    pub fn every(mut self, count: u32, unit: Interval) -> Self {
        self.count = count;
        self.unit = unit;
        self
    }

    pub fn until(mut self, end: NaiveDate) -> Self {
        self.end = Some(end);
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.count == 0 {
            return Err(Error::Invalid("a plan interval must be at least one unit".into()));
        }
        if let Some(end) = self.end
            && end < self.start
        {
            return Err(Error::Invalid(format!(
                "plan ends {end}, before it starts {}",
                self.start
            )));
        }
        Ok(())
    }

    /// The `n`-th occurrence, counting the start as zero.
    ///
    /// Always measured from `start`, never by stepping off the previous occurrence: a plan
    /// starting on the 31st must fire on 28 Feb and again on 31 Mar, which stepping would
    /// ratchet down to the 28th of every later month.
    pub fn nth(&self, n: u32) -> Result<NaiveDate> {
        match self.unit {
            Interval::Week => {
                let days = i64::from(n) * i64::from(self.count) * 7;
                self.start
                    .checked_add_signed(Duration::days(days))
                    .ok_or_else(|| Error::Math(format!("cannot add {days} days to {}", self.start)))
            }
            Interval::Month => add_months(self.start, n.saturating_mul(self.count)),
        }
    }

    /// Every occurrence inside `[from, to]`, honouring the schedule's own end.
    pub fn occurrences(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<NaiveDate>> {
        self.validate()?;
        let last = match self.end {
            Some(end) => end.min(to),
            None => to,
        };
        let mut out = Vec::new();
        let mut n = 0u32;
        loop {
            let date = self.nth(n)?;
            if date > last {
                break;
            }
            if date >= from {
                out.push(date);
            }
            n += 1;
        }
        Ok(out)
    }

    /// First occurrence strictly after `date`, or `None` once the plan has run out.
    pub fn next_after(&self, date: NaiveDate) -> Result<Option<NaiveDate>> {
        self.validate()?;
        let mut n = 0u32;
        loop {
            let next = self.nth(n)?;
            if next > date {
                return Ok(match self.end {
                    Some(end) if next > end => None,
                    _ => Some(next),
                });
            }
            if self.end.is_some_and(|end| next >= end) {
                return Ok(None);
            }
            n += 1;
        }
    }
}

/// One instrument the contribution is split into. `weight` is a share of the plan's amount,
/// read against the sum of the plan's weights — see ADR-0033.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanLeg {
    pub security_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
}

/// A regular contribution: money on a schedule, optionally split into instruments.
///
/// A plan proposes transactions and never writes them — `calc::plans` builds drafts, the user
/// commits them, and `plan_executions` records which occurrence produced which transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentPlan {
    pub id: String,
    pub portfolio_id: String,
    /// Securities account for a plan with legs, deposit account for a cash contribution plan.
    pub account_id: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: Currency,
    /// Flat cost per occurrence, charged once however many legs the plan has.
    #[serde(with = "rust_decimal::serde::str")]
    pub fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes: Decimal,
    pub schedule: Schedule,
    pub active: bool,
    pub note: Option<String>,
    /// Empty means the plan only moves money onto the deposit account.
    pub legs: Vec<PlanLeg>,
}

impl InvestmentPlan {
    pub fn new(
        portfolio_id: &str,
        account_id: &str,
        name: impl Into<String>,
        amount: Decimal,
        currency: &str,
        schedule: Schedule,
    ) -> Self {
        InvestmentPlan {
            id: super::new_id(),
            portfolio_id: portfolio_id.to_string(),
            account_id: account_id.to_string(),
            name: name.into(),
            amount,
            currency: normalize_currency(currency),
            fees: Decimal::ZERO,
            taxes: Decimal::ZERO,
            schedule,
            active: true,
            note: None,
            legs: Vec::new(),
        }
    }

    pub fn with_leg(mut self, security_id: &str, weight: Decimal) -> Self {
        self.legs.push(PlanLeg {
            security_id: security_id.to_string(),
            weight,
        });
        self
    }

    pub fn with_costs(mut self, fees: Decimal, taxes: Decimal) -> Self {
        self.fees = fees;
        self.taxes = taxes;
        self
    }

    /// No legs: the contribution lands as cash and buys nothing.
    pub fn is_cash_only(&self) -> bool {
        self.legs.is_empty()
    }

    pub fn total_weight(&self) -> Decimal {
        self.legs.iter().map(|l| l.weight).sum()
    }

    /// A leg's share of the contribution. Divides by the sum of the weights, so a plan written
    /// as `6 / 4` splits the same way as one written as `0.6 / 0.4`.
    pub fn leg_share(&self, leg: &PlanLeg) -> Result<Decimal> {
        let total = self.total_weight();
        if total.is_zero() {
            return Err(Error::Invalid(format!(
                "plan {} has no weight to divide",
                self.id
            )));
        }
        Ok(leg.weight / total)
    }

    /// The gross amount that goes into one leg, before costs.
    pub fn leg_amount(&self, leg: &PlanLeg) -> Result<Decimal> {
        Ok(self.amount * self.leg_share(leg)?)
    }

    pub fn validate(&self) -> Result<()> {
        self.schedule.validate()?;
        if self.amount <= Decimal::ZERO {
            return Err(Error::Invalid(format!(
                "plan {} contributes {}, which is not a positive amount",
                self.id, self.amount
            )));
        }
        if self.fees < Decimal::ZERO || self.taxes < Decimal::ZERO {
            return Err(Error::Invalid(format!("plan {} has negative costs", self.id)));
        }
        for leg in &self.legs {
            if leg.weight <= Decimal::ZERO {
                return Err(Error::Invalid(format!(
                    "plan leg {} has weight {}, which is not positive",
                    leg.security_id, leg.weight
                )));
            }
        }
        let mut seen: Vec<&str> = self.legs.iter().map(|l| l.security_id.as_str()).collect();
        seen.sort_unstable();
        if seen.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::Invalid(format!(
                "plan {} names the same instrument twice",
                self.id
            )));
        }
        if !self.is_cash_only() && self.fees + self.taxes >= self.amount {
            return Err(Error::Invalid(format!(
                "plan {} spends its whole contribution on costs",
                self.id
            )));
        }
        Ok(())
    }
}

/// `n` months on, clamped to the length of the target month (31 Jan + 1 month = 28 Feb).
fn add_months(date: NaiveDate, n: u32) -> Result<NaiveDate> {
    let total = date.month0() as i64 + i64::from(n);
    let year = date.year() as i64 + total / 12;
    let month = (total % 12) as u32 + 1;
    let year = i32::try_from(year).map_err(|_| Error::Math(format!("cannot add {n} months to {date}")))?;
    let last = last_day_of_month(year, month)?;
    NaiveDate::from_ymd_opt(year, month, date.day().min(last))
        .ok_or_else(|| Error::Math(format!("cannot add {n} months to {date}")))
}

fn last_day_of_month(year: i32, month: u32) -> Result<u32> {
    let first_next = match month {
        12 => NaiveDate::from_ymd_opt(year + 1, 1, 1),
        _ => NaiveDate::from_ymd_opt(year, month + 1, 1),
    };
    first_next
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .ok_or_else(|| Error::Math(format!("no month {month} in {year}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn monthly_plan_keeps_the_day_of_month() {
        // Start 31 Jan 2024 (a leap year): Feb has 29 days, Mar has 31. Measuring every
        // occurrence from the start rather than the previous one keeps the 31st.
        let s = Schedule::monthly(d("2024-01-31"));
        assert_eq!(s.nth(0).unwrap(), d("2024-01-31"));
        assert_eq!(s.nth(1).unwrap(), d("2024-02-29"));
        assert_eq!(s.nth(2).unwrap(), d("2024-03-31"));
        assert_eq!(s.nth(3).unwrap(), d("2024-04-30"));
    }

    #[test]
    fn quarterly_is_three_months() {
        let s = Schedule::monthly(d("2024-01-15")).every(3, Interval::Month);
        assert_eq!(s.nth(1).unwrap(), d("2024-04-15"));
        assert_eq!(s.nth(4).unwrap(), d("2025-01-15"));
    }

    #[test]
    fn weekly_steps_seven_days() {
        let s = Schedule {
            start: d("2024-06-03"),
            end: None,
            unit: Interval::Week,
            count: 2,
        };
        assert_eq!(s.nth(1).unwrap(), d("2024-06-17"));
    }

    #[test]
    fn occurrences_are_clipped_by_the_window_and_the_end() {
        let s = Schedule::monthly(d("2024-01-10")).until(d("2024-05-01"));
        // Occurrences: 10 Jan, 10 Feb, 10 Mar, 10 Apr; the plan ends 1 May, the window starts 1 Feb.
        let got = s.occurrences(d("2024-02-01"), d("2024-12-31")).unwrap();
        assert_eq!(got, vec![d("2024-02-10"), d("2024-03-10"), d("2024-04-10")]);
    }

    #[test]
    fn next_after_stops_at_the_end() {
        let s = Schedule::monthly(d("2024-01-10")).until(d("2024-03-01"));
        assert_eq!(s.next_after(d("2024-01-10")).unwrap(), Some(d("2024-02-10")));
        assert_eq!(s.next_after(d("2024-02-10")).unwrap(), None);
    }

    #[test]
    fn leg_amount_divides_by_the_sum_of_weights() {
        // 500 split 6 / 4 must equal 500 split 0.6 / 0.4: 500 * 6 / 10 = 300.
        let plan = InvestmentPlan::new(
            "p",
            "acc",
            "Monthly",
            dec!(500),
            "EUR",
            Schedule::monthly(d("2024-01-01")),
        )
        .with_leg("a", dec!(6))
        .with_leg("b", dec!(4));
        assert_eq!(plan.leg_amount(&plan.legs[0]).unwrap(), dec!(300));
        assert_eq!(plan.leg_amount(&plan.legs[1]).unwrap(), dec!(200));
    }

    #[test]
    fn a_plan_cannot_name_one_instrument_twice() {
        let plan = InvestmentPlan::new(
            "p",
            "acc",
            "Monthly",
            dec!(500),
            "EUR",
            Schedule::monthly(d("2024-01-01")),
        )
        .with_leg("a", dec!(1))
        .with_leg("a", dec!(1));
        assert!(plan.validate().is_err());
    }

    #[test]
    fn costs_may_not_swallow_the_contribution() {
        let plan = InvestmentPlan::new(
            "p",
            "acc",
            "Monthly",
            dec!(10),
            "EUR",
            Schedule::monthly(d("2024-01-01")),
        )
        .with_leg("a", dec!(1))
        .with_costs(dec!(10), Decimal::ZERO);
        assert!(plan.validate().is_err());
    }

    #[test]
    fn a_cash_plan_has_no_legs_and_still_validates() {
        let plan = InvestmentPlan::new(
            "p",
            "cash",
            "Salary",
            dec!(1000),
            "EUR",
            Schedule::monthly(d("2024-01-01")),
        );
        assert!(plan.is_cash_only());
        plan.validate().unwrap();
    }
}
