use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};
use sq_core::market::DateRange;

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

pub(super) const PLAN_COMMIT: Tool = Tool {
    name: "plan_commit",
    description: "Record one contribution a plan already owes, exactly as the plan proposes \
                  it. Use plans_list first to see which plans have occurrences waiting. This \
                  writes real transactions; if the broker filled a different price, record \
                  them with transaction_create instead.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "plan": { "type": "string", "description": "The plan's name, as plans_list returned it." },
                "date": { "type": ["string", "null"], "description": "Which occurrence, YYYY-MM-DD. Null means the oldest one still owed." }
            },
            "required": ["plan", "date"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("plan".into(), text(args, "plan"));
        params.insert("occurrence".into(), or_all(args, "date"));
        params
    },
    run: plan_commit,
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

pub(super) const PLAN_SAVE: Tool = Tool {
    name: "plan_save",
    description: "Write an investment plan or change one: what it contributes, how often, from \
                  which account and into which instruments. A plan only proposes purchases — \
                  nothing is recorded in the ledger until plan_commit. The legs given are the \
                  whole plan afterwards; no legs means the contribution simply lands as cash.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "plan": { "type": "string", "description": "The plan's name. A name that does not exist yet creates it." },
                "account": { "type": ["string", "null"], "description": "Which account it runs on: a securities account when it buys, a deposit account when it only saves cash. Required for a new plan." },
                "amount": { "type": ["string", "null"], "description": "What one occurrence contributes. Required for a new plan." },
                "currency": { "type": ["string", "null"], "description": "The contribution's currency. Null means the account's own." },
                "every": { "type": ["integer", "null"], "description": "How many units between occurrences: 1 with MONTH is monthly, 3 is quarterly." },
                "unit": { "type": ["string", "null"], "enum": ["WEEK", "MONTH", null], "description": "The unit of that interval. Null means months." },
                "start": { "type": ["string", "null"], "description": "The first occurrence, YYYY-MM-DD. Null means today for a new plan." },
                "end": { "type": ["string", "null"], "description": "The last occurrence, YYYY-MM-DD. Null means it runs until stopped." },
                "legs": {
                    "type": ["array", "null"],
                    "description": "What each occurrence buys. Null leaves an existing plan's instruments alone; an empty list makes it a cash contribution.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "symbol": { "type": "string", "description": "The instrument's ticker." },
                            "percent": { "type": "string", "description": "Its share of the contribution, 0 to 100." }
                        },
                        "required": ["symbol", "percent"],
                        "additionalProperties": false
                    }
                },
                "fees": { "type": ["string", "null"], "description": "Flat cost per occurrence, charged once however many instruments it buys." },
                "active": { "type": ["boolean", "null"], "description": "False pauses the plan without deleting it." }
            },
            "required": ["plan", "account", "amount", "currency", "every", "unit", "start", "end", "legs", "fees", "active"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("plan".into(), text(args, "plan"));
        for key in ["account", "amount", "currency", "start", "end", "fees"] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        if let Some(every) = args.get("every").and_then(Value::as_u64) {
            params.insert(
                "every".into(),
                format!(
                    "{every} {}",
                    optional(args, "unit").unwrap_or_else(|| "MONTH".into())
                ),
            );
        }
        let legs = legs_text(args);
        if !legs.is_empty() {
            params.insert("buys".into(), legs);
        }
        params
    },
    run: plan_save,
};

pub(super) const PLAN_DELETE: Tool = Tool {
    name: "plan_delete",
    description: "Delete an investment plan. The purchases it already recorded stay in the \
                  ledger — they happened. To stop a plan without losing it, set it inactive \
                  with plan_save instead.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "plan": { "type": "string", "description": "The plan's name, as plans_list returned it." }
            },
            "required": ["plan"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("plan", text(args, "plan")),
    run: plan_delete,
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

/// Plans, alerts and watchlists are **not** scoped (`.claude/rules/ui-boundary.md`): an intention
/// about the portfolio, a level on an instrument and a list of instruments do not change when the
/// account picker narrows, so these bodies read the portfolio rather than the lens.
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

/// The body of `plan_commit`: the rows the plan proposes and the link that records the occurrence
/// as done, written together. Half of it would leave the plan offering a month whose purchases
/// are already in the ledger (ADR-0033).
pub(super) fn plan_commit(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let plan = plan_by_name(context, &text(args, "plan"))?;

    let executed = context.store.plan_executions(&plan.id).map_err(tool)?;
    let due = sq_core::calc::due_occurrences(&plan, &executed, context.today).map_err(tool)?;
    let date = match date_argument(args, "date")? {
        Some(date) => {
            if !due.contains(&date) {
                return Err(AiError::Tool(format!(
                    "{} owes nothing on {date}; it owes {}",
                    plan.name,
                    due.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ")
                )));
            }
            date
        }
        None => *due
            .first()
            .ok_or_else(|| AiError::Tool(format!("{} owes nothing yet", plan.name)))?,
    };

    let securities = context.store.list_securities().map_err(tool)?;
    let occurrence = sq_core::calc::plan_occurrence(&plan, date, &securities, context.store, context.store)
        .map_err(tool)?;
    let transactions = sq_core::calc::plan_transactions(&plan, &occurrence);

    let mut ids = Vec::with_capacity(transactions.len());
    for transaction in &transactions {
        context.store.save_transaction(transaction).map_err(tool)?;
        ids.push(transaction.id.clone());
    }
    context
        .store
        .record_plan_execution(&plan.id, date, &ids)
        .map_err(tool)?;
    (context.changed)("transactions");

    Ok(json!({
        "plan": plan.name,
        "occurrence": date.to_string(),
        "written": transactions.len(),
        "amount": money(occurrence.amount),
        "currency": occurrence.currency,
        "still_owed": due.len() - 1,
    }))
}

/// The legs as the card shows them: the instruments and their shares, in the order given.
fn legs_text(args: &Value) -> String {
    args.get("legs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| format!("{} {}%", text(item, "symbol"), text(item, "percent")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// The body of `plan_due`: what the plan proposes, priced at the occurrence's own date. A
/// missing price or rate is reported as a problem on that occurrence, not as a failed call —
/// the other occurrences are still answerable (ADR-0033).
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

/// The body of `plan_save`. A plan writes nothing by itself — this only changes the intention,
/// and `plan_commit` is still what puts rows in the ledger (ADR-0033).
pub(super) fn plan_save(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let name = text(args, "plan").trim().to_string();
    if name.is_empty() {
        return Err(AiError::Tool("a plan needs a name".into()));
    }
    let existing = plan_by_name(context, &name).ok();

    let account = match optional(args, "account") {
        Some(wanted) => account_by_name(context, &wanted)?,
        None => match &existing {
            Some(plan) => context.store.get_account(&plan.account_id).map_err(tool)?,
            None => return Err(AiError::Tool("a new plan needs the account it runs on".into())),
        },
    };
    let amount = match decimal_argument(args, "amount")? {
        Some(amount) => amount,
        None => match &existing {
            Some(plan) => plan.amount,
            None => return Err(AiError::Tool("a new plan needs the amount it contributes".into())),
        },
    };

    let schedule = sq_core::model::Schedule {
        start: date_argument(args, "start")?
            .or(existing.as_ref().map(|p| p.schedule.start))
            .unwrap_or(context.today),
        end: date_argument(args, "end")?.or(existing.as_ref().and_then(|p| p.schedule.end)),
        unit: match optional(args, "unit").as_deref() {
            Some("WEEK") => sq_core::model::Interval::Week,
            Some("MONTH") => sq_core::model::Interval::Month,
            _ => existing
                .as_ref()
                .map(|p| p.schedule.unit)
                .unwrap_or(sq_core::model::Interval::Month),
        },
        count: args
            .get("every")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .or(existing.as_ref().map(|p| p.schedule.count))
            .unwrap_or(1),
    };

    // Absent legs leave an existing plan's instruments alone; an empty list is the user saying
    // the contribution should buy nothing, which is a cash plan rather than a broken one.
    let legs = match args.get("legs") {
        Some(Value::Array(items)) => {
            let mut legs = Vec::new();
            for item in items {
                let security = security_by_symbol(context, &text(item, "symbol"))?;
                let share = decimal_argument(item, "percent")?
                    .ok_or_else(|| AiError::Tool(format!("{} has no share", security.symbol)))?;
                legs.push(sq_core::model::PlanLeg {
                    security_id: security.id,
                    weight: share / rust_decimal::Decimal::ONE_HUNDRED,
                });
            }
            legs
        }
        _ => existing.as_ref().map(|p| p.legs.clone()).unwrap_or_default(),
    };

    let plan = sq_core::model::InvestmentPlan {
        id: existing
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(sq_core::model::new_id),
        portfolio_id: context.scope.portfolio.id.clone(),
        account_id: account.id.clone(),
        name,
        amount,
        currency: sq_core::money::normalize_currency(
            &optional(args, "currency")
                .or_else(|| existing.as_ref().map(|p| p.currency.clone()))
                .unwrap_or_else(|| account.currency.clone()),
        ),
        fees: decimal_argument(args, "fees")?
            .or(existing.as_ref().map(|p| p.fees))
            .unwrap_or_default(),
        taxes: existing.as_ref().map(|p| p.taxes).unwrap_or_default(),
        schedule,
        active: args
            .get("active")
            .and_then(Value::as_bool)
            .or(existing.as_ref().map(|p| p.active))
            .unwrap_or(true),
        note: existing.as_ref().and_then(|p| p.note.clone()),
        legs,
    };
    plan.validate().map_err(tool)?;
    context.store.save_plan(&plan).map_err(tool)?;
    (context.changed)("plans");

    let securities = context.store.list_securities().map_err(tool)?;
    Ok(json!({
        "plan": plan.name,
        "created": existing.is_none(),
        "account": account.name,
        "amount": money(plan.amount),
        "currency": plan.currency,
        "every": format!("{} {:?}", plan.schedule.count, plan.schedule.unit),
        "starts": plan.schedule.start.to_string(),
        "active": plan.active,
        "buys": plan
            .legs
            .iter()
            .map(|leg| json!({
                "instrument": securities
                    .iter()
                    .find(|s| s.id == leg.security_id)
                    .map(|s| s.symbol.as_str())
                    .unwrap_or("?"),
                "share_percent": plan.leg_share(leg).map(percent).unwrap_or(Value::Null),
            }))
            .collect::<Vec<_>>(),
        "next_date": plan.schedule.next_after(context.today).map_err(tool)?.map(|d| d.to_string()),
    }))
}

pub(super) fn plan_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let plan = plan_by_name(context, &text(args, "plan"))?;
    let executed = context.store.plan_executions(&plan.id).map_err(tool)?.len();
    context.store.delete_plan(&plan.id).map_err(tool)?;
    (context.changed)("plans");
    // The rows it already wrote are the ledger's, not the plan's, and they stay.
    Ok(json!({ "removed": plan.name, "committed_occurrences_kept": executed }))
}
