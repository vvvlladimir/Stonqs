use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};
use sq_core::market::DateRange;

mod write;
pub(super) use write::{PLAN_COMMIT, PLAN_DELETE, PLAN_SAVE};

pub(super) const PLANS_LIST: Tool = Tool {
    name: "plans_list",
    description: "The user's investment plans: what each buys, how often, for how much, when \
                  it next fires and how many occurrences are waiting to be committed. Plans \
                  are about the whole portfolio and ignore the account lens.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: plans_list,
};

pub(super) const PLANS_GOALS: Tool = Tool {
    name: "plans_goals",
    description: "The user's savings goals: the amount each is for, what its accounts are \
                  worth now, how far along it is, and either what it would take per month to \
                  arrive on time or when the stated pace arrives. A goal names its own \
                  accounts and ignores the account lens.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: plans_goals,
};

pub(super) const ACCOUNTS_LIMITS: Tool = Tool {
    name: "accounts_limits",
    description: "Yearly contribution ceilings the user stated for their accounts — an ISA, a \
                  401(k), an ИИС — and what has been paid into each over its own limit year. \
                  The app enforces nothing and knows no country's rules: these are the user's \
                  own figures.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: accounts_limits,
};

pub(super) const PLANS_PROJECTION: Tool = Tool {
    name: "plans_projection",
    description: "What the active plans would contribute month by month over the coming \
                  months, at today's exchange rates. Use it for \"how much am I putting in\".",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "months": { "type": ["integer", "null"], "description": "How far ahead to project. Null means 12." }
            },
            "required": ["months"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("months", months_of(args).to_string()),
    run: plans_projection,
};

pub(super) const PLANS_DUE: Tool = Tool {
    name: "plans_due",
    description: "The contributions a plan owes but has not recorded yet, each with the exact \
                  rows it would write. Call it before plan_commit — an occurrence whose price \
                  or exchange rate is missing says so here instead of failing the commit.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "plan": { "type": ["string", "null"], "description": "One plan by name. Null covers every plan." }
            },
            "required": ["plan"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("plan", or_all(args, "plan")),
    run: plans_due,
};

/// How far ahead a projection reaches when the model does not say. A year is what the plans
/// screen opens with, and it is the window a contribution question is usually about.
pub(super) const PROJECTION_MONTHS: u32 = 12;
fn months_of(args: &Value) -> u32 {
    args.get("months")
        .and_then(Value::as_u64)
        .map(|n| n.clamp(1, 120) as u32)
        .unwrap_or(PROJECTION_MONTHS)
}

/// Not scoped: plans, alerts and watchlists read the portfolio, not the lens.
pub(super) fn plans_goals(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let portfolio = &context.scope.portfolio;
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let accounts = context.store.list_accounts().map_err(tool)?;

    let mut rows = Vec::new();
    for goal in context.store.list_goals(&portfolio.id).map_err(tool)? {
        let progress = analytics.goal_progress(&goal, context.today).map_err(tool)?;
        let names: Vec<&str> = goal
            .accounts
            .iter()
            .filter_map(|id| accounts.iter().find(|a| &a.id == id))
            .map(|a| a.name.as_str())
            .collect();
        rows.push(json!({
            "name": goal.name,
            "target": money(progress.target_base),
            "current": money(progress.current_base),
            "missing": money(progress.missing_base),
            "progress_percent": percent(progress.progress),
            "accounts": if names.is_empty() { Value::String("the whole portfolio".into()) } else { json!(names) },
            "target_date": goal.target_date.map(|d| d.to_string()),
            "months_left": progress.months_left,
            "required_monthly": progress.required_monthly_base.map(money),
            "stated_monthly": progress.monthly_base.map(money),
            // Absent means the question cannot be answered, which is not the same as "behind".
            "on_track": progress.on_track,
            "arrives": progress.projected_date.map(|d| d.to_string()),
        }));
    }

    Ok(json!({
        "base_currency": portfolio.base_currency,
        "goals": rows,
    }))
}

pub(super) fn accounts_limits(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let accounts = context.store.list_accounts().map_err(tool)?;

    let mut rows = Vec::new();
    for limit in context.store.list_limits().map_err(tool)? {
        let usage = analytics.limit_usage(&limit, context.today).map_err(tool)?;
        rows.push(json!({
            "name": usage.name,
            "account": accounts
                .iter()
                .find(|a| a.id == usage.account_id)
                .map(|a| a.name.as_str())
                .unwrap_or("?"),
            "year_from": usage.from.to_string(),
            "year_to": usage.to.to_string(),
            "allowance": money(usage.allowance),
            "used": money(usage.used),
            "remaining": money(usage.remaining),
            "used_percent": percent(usage.share),
            "currency": usage.currency,
        }));
    }

    Ok(json!({ "limits": rows }))
}

pub(super) fn plans_list(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let portfolio = &context.scope.portfolio;
    let plans = context.store.list_plans(&portfolio.id).map_err(tool)?;
    let accounts = context.store.list_accounts().map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let rates = plan_rates(context, &plans)?;
    let monthly =
        sq_core::calc::monthly_contribution(&plans, &portfolio.base_currency, &rates, context.today)
            .map_err(tool)?;

    let mut rows = Vec::with_capacity(plans.len());
    for plan in &plans {
        let executed = context.store.plan_executions(&plan.id).map_err(tool)?;
        let legs: Vec<Value> = plan
            .legs
            .iter()
            .map(|leg| {
                let security = securities.iter().find(|s| s.id == leg.security_id);
                json!({
                    "instrument": security.map(|s| s.symbol.as_str()).unwrap_or("?"),
                    "name": security.map(|s| s.name.as_str()).unwrap_or("?"),
                    "share_percent": plan.leg_share(leg).map(percent).unwrap_or(Value::Null),
                })
            })
            .collect();

        rows.push(json!({
            "name": plan.name,
            "active": plan.active,
            "amount": money(plan.amount),
            "currency": plan.currency,
            "every": format!("{} {:?}", plan.schedule.count, plan.schedule.unit),
            "account": accounts
                .iter()
                .find(|a| a.id == plan.account_id)
                .map(|a| a.name.as_str())
                .unwrap_or("?"),
            // No legs is a cash contribution plan, not a broken one (ADR-0033).
            "buys": legs,
            "next_date": plan.schedule.next_after(context.today).map_err(tool)?.map(|d| d.to_string()),
            "due_occurrences": sq_core::calc::due_occurrences(plan, &executed, context.today)
                .map_err(tool)?
                .len(),
            "last_executed": executed.iter().next_back().map(|d| d.to_string()),
        }));
    }

    Ok(json!({
        "base_currency": portfolio.base_currency,
        "monthly_total": money(monthly),
        "plans": rows,
    }))
}

pub(super) fn plans_projection(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let portfolio = &context.scope.portfolio;
    let plans = context.store.list_plans(&portfolio.id).map_err(tool)?;
    let months = months_of(args);
    let to = context
        .today
        .checked_add_months(chrono::Months::new(months))
        .ok_or_else(|| AiError::Tool("the projection window runs off the calendar".into()))?;

    let rates = plan_rates(context, &plans)?;
    // Every leg is converted at today's rate: a future date has no rate, so the answer is
    // explicitly "at today's rates" rather than a forecast of them.
    let contributions = sq_core::calc::contribution_schedule(
        &plans,
        DateRange::new(context.today, to),
        &portfolio.base_currency,
        &rates,
        context.today,
    )
    .map_err(tool)?;
    let by_month = sq_core::calc::contributions_by_month(&contributions);
    let total: rust_decimal::Decimal = contributions.iter().map(|c| c.amount_base).sum();

    Ok(json!({
        "from": context.today.to_string(),
        "to": to.to_string(),
        "base_currency": portfolio.base_currency,
        "total": money(total),
        "contributions": contributions.len(),
        "by_month": by_month
            .iter()
            .map(|(month, amount)| json!({ "month": month, "total": money(*amount) }))
            .collect::<Vec<_>>(),
    }))
}

/// The pairs `monthly_contribution` and `contribution_schedule` need, read once per call.
pub(super) fn plan_rates(
    context: &ToolContext,
    plans: &[sq_core::model::InvestmentPlan],
) -> AiResult<sq_core::fx::RateCache> {
    let base = &context.scope.portfolio.base_currency;
    let pairs: Vec<(String, String)> = plans
        .iter()
        .map(|p| (p.currency.clone(), base.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    context.store.rate_cache(&pairs, context.today).map_err(tool)
}

/// Priced at each occurrence's date; a missing price is a problem on that occurrence, not a failed call.
pub(super) fn plans_due(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let plans = match optional(args, "plan") {
        Some(name) => vec![plan_by_name(context, &name)?],
        None => context
            .store
            .list_plans(&context.scope.portfolio.id)
            .map_err(tool)?,
    };
    let securities = context.store.list_securities().map_err(tool)?;

    let mut rows = Vec::new();
    for plan in &plans {
        let executed = context.store.plan_executions(&plan.id).map_err(tool)?;
        for date in sq_core::calc::due_occurrences(plan, &executed, context.today).map_err(tool)? {
            let drafted =
                sq_core::calc::plan_occurrence(plan, date, &securities, context.store, context.store);
            rows.push(match drafted {
                Ok(occurrence) => {
                    let transactions = sq_core::calc::plan_transactions(plan, &occurrence);
                    json!({
                        "plan": plan.name,
                        "occurrence": date.to_string(),
                        "amount": money(occurrence.amount),
                        "currency": occurrence.currency,
                        "rows": transactions
                            .iter()
                            .map(|t| json!({
                                "kind": format!("{:?}", t.kind),
                                "instrument": t
                                    .security_id
                                    .as_ref()
                                    .and_then(|id| securities.iter().find(|s| &s.id == id))
                                    .map(|s| s.symbol.as_str()),
                                "quantity": quantity(t.quantity),
                                "price": price(t.price),
                                "amount": money(t.amount),
                            }))
                            .collect::<Vec<_>>(),
                    })
                }
                Err(sq_core::Error::MissingMarketData { kind, .. }) => json!({
                    "plan": plan.name,
                    "occurrence": date.to_string(),
                    // Nothing can be drafted without it, so the occurrence waits rather than
                    // being priced at a number nobody quoted.
                    "waiting_for": kind,
                }),
                Err(other) => return Err(tool(other)),
            });
        }
    }

    Ok(json!({ "due": rows.len(), "occurrences": rows }))
}
