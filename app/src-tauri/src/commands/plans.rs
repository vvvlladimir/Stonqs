use crate::commands::parse_date;
use crate::commands::transactions::{TransactionInput, decimal, from_input};
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::{
    Contribution, FireAssumptions, FireProjection, PlanOccurrence, contribution_schedule,
    contributions_by_month, due_occurrences, monthly_contribution, plan_occurrence, plan_transactions,
};
use sq_core::market::DateRange;
use sq_core::prelude::*;
use std::collections::BTreeMap;
use tauri::{AppHandle, State};

/// A leg with the instrument named and its share worked out, so the list reads without a join.
#[derive(Debug, Serialize)]
pub struct PlanLegRow {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
    /// `weight` over the plan's total, which is the share the money is actually split by.
    #[serde(with = "rust_decimal::serde::str")]
    pub share: Decimal,
}

#[derive(Debug, Serialize)]
pub struct PlanRow {
    pub plan: InvestmentPlan,
    pub account_name: String,
    pub legs: Vec<PlanLegRow>,
    /// Next date the plan fires, whether or not anything is owed yet.
    pub next_date: Option<String>,
    /// Occurrences up to today with nothing committed against them.
    pub due_count: usize,
    pub last_executed: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlansData {
    pub base_currency: String,
    pub rows: Vec<PlanRow>,
    /// What every active plan adds up to in an average month, over the coming year.
    #[serde(with = "rust_decimal::serde::str")]
    pub monthly_base: Decimal,
}

/// One occurrence a plan still owes, with the draft it would write.
///
/// `problem` is a code, never a sentence: the frontend owns the wording (ADR-0023). A date with
/// no quote yet keeps its row so the screen can say which month is stuck rather than failing.
#[derive(Debug, Serialize)]
pub struct PlanDue {
    pub date: String,
    pub occurrence: Option<PlanOccurrence>,
    /// The rows the plan would write, already shaped like anything the editor saves — the
    /// frontend shows and edits them, it never derives a quantity of its own.
    pub drafts: Vec<Transaction>,
    pub problem: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlanProjection {
    pub base_currency: String,
    pub contributions: Vec<Contribution>,
    /// `YYYY-MM` -> base-currency total, ready for a bar per month.
    pub by_month: BTreeMap<String, Decimal>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_base: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct PlanLegInput {
    pub security_id: String,
    pub weight: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanInput {
    pub id: Option<String>,
    pub account_id: String,
    pub name: String,
    pub amount: String,
    pub currency: String,
    pub fees: Option<String>,
    pub taxes: Option<String>,
    pub start: String,
    pub end: Option<String>,
    pub interval_unit: Interval,
    pub interval_count: u32,
    pub active: bool,
    pub note: Option<String>,
    pub legs: Vec<PlanLegInput>,
}

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Every plan of the portfolio. Not scoped: a plan is an intention about the portfolio, and
/// narrowing the account picker must not make a contribution disappear from the list.
#[tauri::command]
pub fn plans_list(state: State<AppState>) -> UiResult<PlansData> {
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let plans = store.list_plans(&portfolio.id)?;
    let accounts = store.list_accounts()?;
    let securities = store.list_securities()?;
    let as_of = today();

    let pairs: Vec<(String, String)> = plans
        .iter()
        .map(|p| (p.currency.clone(), portfolio.base_currency.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let rates = store.rate_cache(&pairs, as_of)?;
    let monthly_base = monthly_contribution(&plans, &portfolio.base_currency, &rates, as_of)?;

    let mut rows = Vec::with_capacity(plans.len());
    for plan in plans {
        let executed = store.plan_executions(&plan.id)?;
        let legs = plan
            .legs
            .iter()
            .map(|leg| {
                let security = securities.iter().find(|s| s.id == leg.security_id);
                Ok(PlanLegRow {
                    security_id: leg.security_id.clone(),
                    symbol: security.map(|s| s.symbol.clone()).unwrap_or_default(),
                    name: security.map(|s| s.name.clone()).unwrap_or_default(),
                    weight: leg.weight,
                    share: plan.leg_share(leg)?,
                })
            })
            .collect::<UiResult<Vec<_>>>()?;

        rows.push(PlanRow {
            account_name: accounts
                .iter()
                .find(|a| a.id == plan.account_id)
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            legs,
            next_date: plan.schedule.next_after(as_of)?.map(|d| d.to_string()),
            due_count: due_occurrences(&plan, &executed, as_of)?.len(),
            last_executed: executed.iter().next_back().map(|d| d.to_string()),
            plan,
        });
    }

    Ok(PlansData {
        base_currency: portfolio.base_currency,
        rows,
        monthly_base,
    })
}

/// What one plan still owes, oldest first, each with the draft transactions it would write.
#[tauri::command]
pub fn plan_due(state: State<AppState>, plan_id: String, as_of: Option<String>) -> UiResult<Vec<PlanDue>> {
    let store = state.store()?;
    let plan = store.get_plan(&plan_id)?;
    let as_of = as_of
        .as_deref()
        .map(parse_date)
        .transpose()?
        .unwrap_or_else(today);
    let executed = store.plan_executions(&plan.id)?;
    let securities = store.list_securities()?;

    due_occurrences(&plan, &executed, as_of)?
        .into_iter()
        .map(|date| {
            Ok(
                match plan_occurrence(&plan, date, &securities, &*store, &*store) {
                    Ok(occurrence) => PlanDue {
                        date: date.to_string(),
                        drafts: plan_transactions(&plan, &occurrence),
                        occurrence: Some(occurrence),
                        problem: None,
                    },
                    Err(sq_core::Error::MissingMarketData { kind, .. }) => PlanDue {
                        date: date.to_string(),
                        occurrence: None,
                        drafts: Vec::new(),
                        problem: Some(match kind {
                            "price" => "MISSING_PRICE".to_string(),
                            _ => "MISSING_RATE".to_string(),
                        }),
                    },
                    Err(other) => return Err(UiError::from(other)),
                },
            )
        })
        .collect()
}

/// Writes the transactions the user confirmed and records the occurrence as executed.
///
/// One command rather than "save each, then mark": a half-written occurrence would leave the
/// plan offering a month whose purchases are already in the ledger.
#[tauri::command]
pub fn plan_commit(
    app: AppHandle,
    state: State<AppState>,
    plan_id: String,
    date: String,
    drafts: Vec<TransactionInput>,
) -> UiResult<usize> {
    let occurrence = parse_date(&date)?;
    if drafts.is_empty() {
        return Err(UiError::invalid(
            "a plan occurrence needs at least one transaction",
        ));
    }
    let transactions = drafts.into_iter().map(from_input).collect::<UiResult<Vec<_>>>()?;

    {
        let store = state.store()?;
        store.get_plan(&plan_id)?;
        for transaction in &transactions {
            store.save_transaction(transaction)?;
        }
        let ids: Vec<String> = transactions.iter().map(|t| t.id.clone()).collect();
        store.record_plan_execution(&plan_id, occurrence, &ids)?;
    }

    crate::jobs::fetch_missing(&app, &state);
    emit_changed(&app, "transactions")?;
    Ok(transactions.len())
}

/// Contributions still to come over the next `months` months, at today's exchange rates.
#[tauri::command]
pub fn plan_projection(state: State<AppState>, months: u32) -> UiResult<PlanProjection> {
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let plans = store.list_plans(&portfolio.id)?;
    let as_of = today();
    let to = as_of
        .checked_add_months(chrono::Months::new(months.clamp(1, 600)))
        .ok_or_else(|| UiError::invalid("the projection window runs off the calendar"))?;

    let pairs: Vec<(String, String)> = plans
        .iter()
        .map(|p| (p.currency.clone(), portfolio.base_currency.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let rates = store.rate_cache(&pairs, as_of)?;

    let contributions = contribution_schedule(
        &plans,
        DateRange::new(as_of, to),
        &portfolio.base_currency,
        &rates,
        as_of,
    )?;
    let by_month = contributions_by_month(&contributions);
    let total_base = contributions.iter().map(|c| c.amount_base).sum();

    Ok(PlanProjection {
        base_currency: portfolio.base_currency,
        contributions,
        by_month,
        total_base,
    })
}

#[tauri::command]
pub fn plan_save(app: AppHandle, state: State<AppState>, input: PlanInput) -> UiResult<InvestmentPlan> {
    let portfolio_id = state.portfolio()?.id.clone();
    let amount = decimal(Some(&input.amount), "amount")?
        .ok_or_else(|| UiError::invalid("a plan needs a contribution amount"))?;
    let currency = super::portfolio::require_currency(&input.currency)?;

    let plan = InvestmentPlan {
        id: input.id.unwrap_or_else(sq_core::model::new_id),
        portfolio_id,
        account_id: input.account_id,
        name: input.name.trim().to_string(),
        amount,
        currency,
        fees: decimal(input.fees.as_deref(), "commission")?.unwrap_or_default(),
        taxes: decimal(input.taxes.as_deref(), "tax")?.unwrap_or_default(),
        schedule: Schedule {
            start: parse_date(&input.start)?,
            end: input.end.as_deref().map(parse_date).transpose()?,
            unit: input.interval_unit,
            count: input.interval_count,
        },
        active: input.active,
        note: input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
        legs: input
            .legs
            .into_iter()
            .map(|leg| {
                Ok(PlanLeg {
                    security_id: leg.security_id,
                    weight: decimal(Some(&leg.weight), "weight")?
                        .ok_or_else(|| UiError::invalid("a plan leg needs a weight"))?,
                })
            })
            .collect::<UiResult<Vec<_>>>()?,
    };

    state.store()?.save_plan(&plan)?;
    emit_changed(&app, "plans")?;
    Ok(plan)
}

#[tauri::command]
pub fn plan_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_plan(&id)?;
    emit_changed(&app, "plans")
}

/// The projection plus the currency every figure in it is in.
#[derive(Debug, Serialize)]
pub struct FireData {
    pub base_currency: String,
    #[serde(flatten)]
    pub projection: FireProjection,
}

/// A figure the widget's form sent; an empty one is not a zero but a missing assumption.
fn amount(value: &str, what: &str) -> UiResult<Decimal> {
    decimal(Some(value), what)?.ok_or_else(|| UiError::invalid(format!("{what} is required")))
}

/// What the portfolio must be worth to pay for a year, and when this pace would get there.
///
/// Not scoped, for the same reason [`plans_list`] is not: the contribution comes from the
/// plans, which belong to the portfolio rather than to a lens, and a target read against one
/// account's value while counting every account's savings would compare two different things.
#[tauri::command]
pub fn fire_projection(
    state: State<AppState>,
    annual_spending: String,
    withdrawal_rate: String,
    expected_return: String,
    contribution: Option<String>,
) -> UiResult<FireData> {
    let store = state.store()?;
    let portfolio = state.portfolio()?.clone();
    let as_of = today();
    let analytics = PortfolioAnalytics::new(&store, &portfolio)?;
    let value = analytics.valuation_at(as_of)?.total_value_base;

    // Left unset, the pace is what the active plans already add up to in a month.
    let monthly = match contribution {
        Some(text) => amount(&text, "monthly contribution")?,
        None => {
            let plans = store.list_plans(&portfolio.id)?;
            let pairs: Vec<(String, String)> = plans
                .iter()
                .map(|p| (p.currency.clone(), portfolio.base_currency.clone()))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let rates = store.rate_cache(&pairs, as_of)?;
            monthly_contribution(&plans, &portfolio.base_currency, &rates, as_of)?
        }
    };

    Ok(FireData {
        projection: sq_core::calc::fire_projection(
            value,
            FireAssumptions {
                annual_spending: amount(&annual_spending, "annual spending")?,
                withdrawal_rate: amount(&withdrawal_rate, "withdrawal rate")?,
                expected_return: amount(&expected_return, "expected return")?,
                monthly_contribution: monthly,
            },
            as_of,
        )?,
        base_currency: portfolio.base_currency,
    })
}
