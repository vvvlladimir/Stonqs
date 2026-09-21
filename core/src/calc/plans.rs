//! Turning an [`InvestmentPlan`] into the transactions it proposes, and into a schedule of
//! contributions still to come. Nothing here writes: a plan proposes, the user commits.
//! See ADR-0033.

use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::{DateRange, PriceLookup};
use crate::model::{InvestmentPlan, Security, Transaction, TransactionKind, floor_to_step};
use crate::money::{Currency, round_money};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One purchase a plan proposes for one occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTrade {
    pub security_id: String,
    pub symbol: String,
    /// Floored to the instrument's tradable step, so it can be zero when the budget is too small.
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Quote price on the occurrence date, forward-filled like every other price lookup.
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    /// Currency of `price` and `amount` — the listing's, not the plan's.
    pub currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    /// The leg's share of the contribution after costs, in the plan's currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub budget: Decimal,
    /// The leg's share of the plan's flat costs, in the plan's currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes: Decimal,
    /// Budget the step rounding left unspent, in the plan's currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_left: Decimal,
    /// Rate from the quote currency to the plan's currency on that date; `1` when they agree.
    #[serde(with = "rust_decimal::serde::str")]
    pub fx_rate: Decimal,
}

/// What one firing of a plan would write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOccurrence {
    pub plan_id: String,
    pub date: NaiveDate,
    /// The account the transactions land on: securities for a plan with legs, deposit otherwise.
    pub account_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: Currency,
    pub trades: Vec<PlannedTrade>,
    /// Contribution left as cash: the whole amount for a cash plan, the step remainder otherwise.
    #[serde(with = "rust_decimal::serde::str")]
    pub cash_left: Decimal,
}

impl PlanOccurrence {
    /// Whether the occurrence buys anything at all. A month whose contribution does not reach
    /// one tradable unit is a real answer, not an error — the money stays cash.
    pub fn buys_nothing(&self) -> bool {
        self.trades.iter().all(|t| t.quantity.is_zero())
    }
}

/// Occurrences up to `as_of` that have not been committed yet, oldest first.
///
/// An inactive plan is due for nothing: stopping a plan must not leave a backlog waiting to be
/// written the moment it is switched on again.
pub fn due_occurrences(
    plan: &InvestmentPlan,
    executed: &BTreeSet<NaiveDate>,
    as_of: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    if !plan.active {
        return Ok(Vec::new());
    }
    Ok(plan
        .schedule
        .occurrences(plan.schedule.start, as_of)?
        .into_iter()
        .filter(|d| !executed.contains(d))
        .collect())
}

/// Builds what the plan would buy on `date`.
///
/// The flat fee and tax are taken off the contribution first — a 500 € debit carrying a 1 € fee
/// buys 499 € of shares — and then divided across the legs by the same weights, so a leg's cost
/// basis carries its own share of the cost rather than the first instrument carrying all of it.
pub fn plan_occurrence(
    plan: &InvestmentPlan,
    date: NaiveDate,
    securities: &[Security],
    prices: &impl PriceLookup,
    rates: &impl RateLookup,
) -> Result<PlanOccurrence> {
    plan.validate()?;
    let mut occurrence = PlanOccurrence {
        plan_id: plan.id.clone(),
        date,
        account_id: plan.account_id.clone(),
        amount: plan.amount,
        currency: plan.currency.clone(),
        trades: Vec::new(),
        cash_left: plan.amount,
    };
    if plan.is_cash_only() {
        return Ok(occurrence);
    }

    let investable = plan.amount - plan.fees - plan.taxes;
    let mut spent = Decimal::ZERO;
    // The last leg absorbs the rounding remainder so the split adds back up to the flat cost.
    let mut fees_left = plan.fees;
    let mut taxes_left = plan.taxes;

    for (index, leg) in plan.legs.iter().enumerate() {
        let last = index + 1 == plan.legs.len();
        let share = plan.leg_share(leg)?;
        let (fees, taxes) = if last {
            (fees_left, taxes_left)
        } else {
            let f = round_money(plan.fees * share);
            let t = round_money(plan.taxes * share);
            (f, t)
        };
        fees_left -= fees;
        taxes_left -= taxes;

        let security = securities
            .iter()
            .find(|s| s.id == leg.security_id)
            .ok_or_else(|| Error::NotFound(format!("security {}", leg.security_id)))?;
        let price = prices
            .price_as_of(&leg.security_id, date)?
            .ok_or_else(|| Error::MissingMarketData {
                kind: "price",
                key: leg.security_id.clone(),
                date,
            })?;
        // The budget is in the plan's currency and the price is in the listing's, so the rate
        // runs quote -> plan. Buying is the one direction we need; `fx_rate` reports it as is.
        let fx_rate = if price.currency == plan.currency {
            Decimal::ONE
        } else {
            rates
                .rate_as_of(&price.currency, &plan.currency, date)?
                .ok_or_else(|| Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{}", price.currency, plan.currency),
                    date,
                })?
        };

        let budget = round_money(investable * share);
        let price_in_plan_currency = price.close * fx_rate;
        let raw = if price_in_plan_currency.is_zero() {
            Decimal::ZERO
        } else {
            budget / price_in_plan_currency
        };
        let quantity = floor_to_step(raw, security.effective_quantity_step());
        let amount = round_money(quantity * price.close);
        let cost_in_plan_currency = round_money(amount * fx_rate);
        spent += cost_in_plan_currency + fees + taxes;

        occurrence.trades.push(PlannedTrade {
            security_id: leg.security_id.clone(),
            symbol: security.symbol.clone(),
            quantity,
            price: price.close,
            currency: price.currency.clone(),
            amount,
            budget,
            fees,
            taxes,
            cash_left: budget - cost_in_plan_currency,
            fx_rate,
        });
    }

    occurrence.cash_left = plan.amount - spent;
    Ok(occurrence)
}

/// The draft transactions for an occurrence, ready to be edited and committed.
///
/// A leg that rounded down to nothing produces no transaction: writing a zero-quantity buy would
/// put a row in the ledger that never happened.
pub fn plan_transactions(plan: &InvestmentPlan, occurrence: &PlanOccurrence) -> Vec<Transaction> {
    if plan.is_cash_only() {
        let mut deposit = Transaction::cash(
            &plan.account_id,
            TransactionKind::Deposit,
            occurrence.date,
            occurrence.amount,
            &occurrence.currency,
        );
        deposit.note = plan.note.clone();
        return vec![deposit];
    }

    occurrence
        .trades
        .iter()
        .filter(|t| !t.quantity.is_zero())
        .map(|t| {
            Transaction::buy(
                &plan.account_id,
                &t.security_id,
                occurrence.date,
                t.quantity,
                t.price,
                &t.currency,
            )
            .with_fees(t.fees)
            .with_taxes(t.taxes)
        })
        .collect()
}

/// One future contribution on the calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub date: NaiveDate,
    pub plan_id: String,
    pub plan_name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
}

/// Contributions every active plan would make inside `range`, oldest first.
///
/// A future date has no exchange rate, so every leg is converted at the rate known on `as_of`:
/// the projection answers "at today's rates", which is the only honest reading of it.
pub fn contribution_schedule(
    plans: &[InvestmentPlan],
    range: DateRange,
    base: &str,
    rates: &impl RateLookup,
    as_of: NaiveDate,
) -> Result<Vec<Contribution>> {
    let mut out = Vec::new();
    for plan in plans.iter().filter(|p| p.active) {
        let rate = if plan.currency == base {
            Decimal::ONE
        } else {
            rates
                .rate_as_of(&plan.currency, base, as_of)?
                .ok_or_else(|| Error::MissingMarketData {
                    kind: "fx rate",
                    key: format!("{}/{base}", plan.currency),
                    date: as_of,
                })?
        };
        for date in plan.schedule.occurrences(range.from, range.to)? {
            out.push(Contribution {
                date,
                plan_id: plan.id.clone(),
                plan_name: plan.name.clone(),
                amount: plan.amount,
                currency: plan.currency.clone(),
                amount_base: round_money(plan.amount * rate),
            });
        }
    }
    out.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.plan_id.cmp(&b.plan_id)));
    Ok(out)
}

/// Base-currency contributions summed per `YYYY-MM`, for a bar per month.
pub fn contributions_by_month(contributions: &[Contribution]) -> BTreeMap<String, Decimal> {
    let mut out: BTreeMap<String, Decimal> = BTreeMap::new();
    for c in contributions {
        *out.entry(c.date.format("%Y-%m").to_string()).or_default() += c.amount_base;
    }
    out
}

/// What the active plans add up to in an average month, in base currency.
///
/// Read off the next twelve months rather than from the interval: a quarterly plan and a monthly
/// one then land on the same scale without anyone dividing by an interval length, and a plan that
/// ends inside the year counts only for the months it still runs.
pub fn monthly_contribution(
    plans: &[InvestmentPlan],
    base: &str,
    rates: &impl RateLookup,
    as_of: NaiveDate,
) -> Result<Decimal> {
    let year = DateRange::new(
        as_of,
        as_of
            .checked_add_months(chrono::Months::new(12))
            .ok_or_else(|| Error::Math(format!("cannot add a year to {as_of}")))?,
    );
    let total: Decimal = contribution_schedule(plans, year, base, rates, as_of)?
        .iter()
        .map(|c| c.amount_base)
        .sum();
    Ok(round_money(total / Decimal::from(12)))
}

/// What the legs of a plan would ask for as a rebalance top-up: the contribution after costs.
/// Feeding it to [`super::rebalance`] as `cash_to_invest` is what makes a plan and a target
/// allocation one screen rather than two.
pub fn investable_amount(plan: &InvestmentPlan) -> Decimal {
    if plan.is_cash_only() {
        plan.amount
    } else {
        plan.amount - plan.fees - plan.taxes
    }
}
