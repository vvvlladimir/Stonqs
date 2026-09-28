//! The plan tools that write: a plan itself, its removal, and one owed occurrence recorded.

use super::super::args::*;
use super::super::fmt::*;
use super::super::lookup::*;
use super::super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use sq_core::model::{Interval, InvestmentPlan, PlanLeg, Schedule};

pub(in crate::ai::tools) const PLAN_COMMIT: Tool = Tool {
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

pub(in crate::ai::tools) const PLAN_SAVE: Tool = Tool {
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

pub(in crate::ai::tools) const PLAN_DELETE: Tool = Tool {
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

/// Rows and the execution link written together (ADR-0033).
fn plan_commit(context: &ToolContext, args: &Value) -> AiResult<Value> {
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

/// The body of `plan_save`. A plan writes nothing by itself — this only changes the intention,
/// and `plan_commit` is still what puts rows in the ledger (ADR-0033).
fn plan_save(context: &ToolContext, args: &Value) -> AiResult<Value> {
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

    let schedule = schedule_of(args, existing.as_ref(), context.today)?;
    let legs = legs_of(context, args, existing.as_ref())?;

    let plan = InvestmentPlan {
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

    saved(context, &plan, &account.name, existing.is_none())
}

/// Every field of the schedule the call leaves out keeps the existing plan's value.
fn schedule_of(args: &Value, existing: Option<&InvestmentPlan>, today: NaiveDate) -> AiResult<Schedule> {
    Ok(Schedule {
        start: date_argument(args, "start")?
            .or(existing.map(|p| p.schedule.start))
            .unwrap_or(today),
        end: date_argument(args, "end")?.or(existing.and_then(|p| p.schedule.end)),
        unit: match optional(args, "unit").as_deref() {
            Some("WEEK") => Interval::Week,
            Some("MONTH") => Interval::Month,
            _ => existing.map(|p| p.schedule.unit).unwrap_or(Interval::Month),
        },
        count: args
            .get("every")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .or(existing.map(|p| p.schedule.count))
            .unwrap_or(1),
    })
}

/// Absent legs leave an existing plan's instruments alone; an empty list is the user saying the
/// contribution should buy nothing, which is a cash plan rather than a broken one.
fn legs_of(context: &ToolContext, args: &Value, existing: Option<&InvestmentPlan>) -> AiResult<Vec<PlanLeg>> {
    let Some(Value::Array(items)) = args.get("legs") else {
        return Ok(existing.map(|p| p.legs.clone()).unwrap_or_default());
    };
    items
        .iter()
        .map(|item| {
            let security = security_by_symbol(context, &text(item, "symbol"))?;
            let share = decimal_argument(item, "percent")?
                .ok_or_else(|| AiError::Tool(format!("{} has no share", security.symbol)))?;
            Ok(PlanLeg {
                security_id: security.id,
                weight: share / Decimal::ONE_HUNDRED,
            })
        })
        .collect()
}

/// The plan as saved, instruments named by ticker.
fn saved(context: &ToolContext, plan: &InvestmentPlan, account: &str, created: bool) -> AiResult<Value> {
    let securities = context.store.list_securities().map_err(tool)?;
    let buys: Vec<Value> = plan
        .legs
        .iter()
        .map(|leg| {
            json!({
                "instrument": securities
                    .iter()
                    .find(|s| s.id == leg.security_id)
                    .map(|s| s.symbol.as_str())
                    .unwrap_or("?"),
                "share_percent": plan.leg_share(leg).map(percent).unwrap_or(Value::Null),
            })
        })
        .collect();
    Ok(json!({
        "plan": plan.name,
        "created": created,
        "account": account,
        "amount": money(plan.amount),
        "currency": plan.currency,
        "every": format!("{} {:?}", plan.schedule.count, plan.schedule.unit),
        "starts": plan.schedule.start.to_string(),
        "active": plan.active,
        "buys": buys,
        "next_date": plan.schedule.next_after(context.today).map_err(tool)?.map(|d| d.to_string()),
    }))
}

fn plan_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let plan = plan_by_name(context, &text(args, "plan"))?;
    let executed = context.store.plan_executions(&plan.id).map_err(tool)?.len();
    context.store.delete_plan(&plan.id).map_err(tool)?;
    (context.changed)("plans");
    // The rows it already wrote are the ledger's, not the plan's, and they stay.
    Ok(json!({ "removed": plan.name, "committed_occurrences_kept": executed }))
}
