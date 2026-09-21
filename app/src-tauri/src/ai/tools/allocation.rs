use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const ALLOCATION_TREES: Tool = Tool {
    name: "allocation_trees",
    description: "The classification trees that exist (asset class, region, sector, or the \
                  user's own). Call this first to learn a tree's name before asking for its \
                  breakdown.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: allocation_trees,
};

pub(super) const ALLOCATION_BREAKDOWN: Tool = Tool {
    name: "allocation_breakdown",
    description: "How the portfolio divides across one classification tree, by name as \
                  listed by allocation_trees. Weights are shares of the classified total.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name, exactly as allocation_trees returned it." }
            },
            "required": ["tree"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("tree", text(args, "tree")),
    run: allocation_breakdown,
};

pub(super) const ALLOCATION_MEMBERS: Tool = Tool {
    name: "allocation_members",
    description: "What sits inside one branch of a classification tree: the instruments and \
                  cash assigned to it, with the share of each that counts. Use it after \
                  allocation_breakdown to answer \"what is in that bucket\".",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name, exactly as allocation_trees returned it." },
                "node": { "type": ["string", "null"], "description": "The branch's name as allocation_breakdown returned it. Null lists every subject of the tree." }
            },
            "required": ["tree", "node"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("tree".into(), text(args, "tree"));
        params.insert("node".into(), or_all(args, "node"));
        params
    },
    run: allocation_members,
};

pub(super) const REBALANCE_TARGETS: Tool = Tool {
    name: "rebalance_targets",
    description: "The target allocations the user has defined, and which tree each one is \
                  written against. Call this first to learn a target's name.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: rebalance_targets,
};

pub(super) const REBALANCE_PLAN: Tool = Tool {
    name: "rebalance_plan",
    description: "How far the portfolio has drifted from one target and what trades would \
                  close the gap. This proposes nothing to the user by itself — it is a \
                  calculation, and only they can act on it.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "description": "The target's name, exactly as rebalance_targets returned it." },
                "cash_to_invest": { "type": ["string", "null"], "description": "New money to spread over the targets, in base currency. Null means none." },
                "allow_sell": { "type": ["boolean", "null"], "description": "Whether overweight positions may be sold. Null means they may." }
            },
            "required": ["target", "cash_to_invest", "allow_sell"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("target".into(), text(args, "target"));
        params.insert("cash".into(), nullable(args, "cash_to_invest"));
        params
    },
    run: rebalance_plan,
};

pub(super) const REBALANCE_TARGET_SAVE: Tool = Tool {
    name: "rebalance_target_save",
    description: "Write a target allocation against a classification tree, or replace one that \
                  exists. Each weight is a share of its own parent branch, not of the whole \
                  portfolio, and the weights of branches sharing a parent must add up to 100. \
                  The weights given are the whole target afterwards.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "description": "The target's name. A name that does not exist yet creates it." },
                "tree": { "type": "string", "description": "The classification tree the weights are written against." },
                "weights": {
                    "type": "array",
                    "description": "Every weighted branch of the target afterwards.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "node": { "type": "string", "description": "The branch's name, as taxonomy_tree returned it." },
                            "percent": { "type": "string", "description": "Its share of its parent branch, 0 to 100." }
                        },
                        "required": ["node", "percent"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["target", "tree", "weights"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("target".into(), text(args, "target"));
        params.insert("tree".into(), text(args, "tree"));
        params.insert("weights".into(), weights_text(args));
        params
    },
    run: rebalance_target_save,
};

pub(super) const REBALANCE_TARGET_DELETE: Tool = Tool {
    name: "rebalance_target_delete",
    description: "Delete a target allocation. The classification tree it was written against \
                  stays as it is.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "description": "The target's name, as rebalance_targets returned it." }
            },
            "required": ["target"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("target", text(args, "target")),
    run: rebalance_target_delete,
};

pub(super) fn allocation_trees(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let trees: Vec<Value> = context
        .store
        .list_taxonomies()
        .map_err(tool)?
        .iter()
        .map(|t| json!({ "name": t.name }))
        .collect();
    Ok(json!({ "trees": trees }))
}

pub(super) fn allocation_breakdown(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;

    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let allocation = analytics
        .allocation_by_taxonomy(&taxonomy.id, context.today)
        .map_err(tool)?;

    Ok(json!({
        "tree": taxonomy.name,
        "date": context.today.to_string(),
        "base_currency": analytics.base_currency(),
        "classified_total": money(allocation.total_base),
        "buckets": allocation
            .buckets
            .iter()
            .map(bucket_json)
            .collect::<Vec<_>>(),
    }))
}

/// Children come along: a tree is asked about once, and a second call per branch would pay the
/// whole valuation again for something already in hand.
pub(super) fn bucket_json(bucket: &sq_core::calc::AllocationBucket) -> Value {
    json!({
        "name": bucket.label,
        "value": money(bucket.value_base),
        "weight_percent": percent(bucket.weight),
        "children": bucket.children.iter().map(bucket_json).collect::<Vec<_>>(),
    })
}

pub(super) fn allocation_members(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let node = match optional(args, "node") {
        None => None,
        Some(wanted) => Some(node_by_name(context, &taxonomy, &wanted)?),
    };

    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let members = analytics
        .allocation_members(&taxonomy.id, node.as_ref().map(|n| n.id.as_str()), context.today)
        .map_err(tool)?;

    Ok(json!({
        "tree": taxonomy.name,
        "node": node.map(|n| n.name),
        "date": context.today.to_string(),
        "base_currency": analytics.base_currency(),
        "members": members
            .iter()
            .map(|member| json!({
                "name": member.symbol,
                "description": member.name,
                "kind": format!("{:?}", member.kind),
                "value": money(member.value_base),
                "weight_percent": percent(member.weight),
                // A subject may be split across branches; this is the part counted here.
                "assigned_share_percent": percent(member.assigned_share),
                // Switched off for this tree: still listed, but out of every percentage.
                "excluded": member.excluded,
            }))
            .collect::<Vec<_>>(),
    }))
}

pub(super) fn rebalance_targets(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let taxonomies = context.store.list_taxonomies().map_err(tool)?;
    let targets: Vec<Value> = context
        .store
        .targets_for_portfolio(&context.scope.portfolio.id)
        .map_err(tool)?
        .iter()
        .map(|target| {
            json!({
                "name": target.name,
                "tree": taxonomies
                    .iter()
                    .find(|t| t.id == target.taxonomy_id)
                    .map(|t| t.name.as_str())
                    .unwrap_or("?"),
                "weighted_branches": target.weights.len(),
            })
        })
        .collect();
    Ok(json!({ "targets": targets }))
}

pub(super) fn rebalance_plan(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let target = target_by_name(context, &text(args, "target"))?;
    let cash = decimal_argument(args, "cash_to_invest")?.unwrap_or(rust_decimal::Decimal::ZERO);
    if cash.is_sign_negative() {
        return Err(AiError::Tool("cash to invest cannot be negative".into()));
    }
    let options = sq_core::calc::RebalanceOptions {
        cash_to_invest: cash,
        allow_sell: args.get("allow_sell").and_then(Value::as_bool).unwrap_or(true),
    };

    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let plan = analytics
        .rebalance(&target, context.today, options)
        .map_err(tool)?;

    Ok(json!({
        "target": target.name,
        "date": context.today.to_string(),
        "base_currency": analytics.base_currency(),
        "total": money(plan.total_base),
        "off_target": money(plan.off_target_base),
        "cash_used": money(plan.cash_used_base),
        "sell_proceeds": money(plan.sell_base),
        "cash_left": money(plan.cash_left_base),
        // Only the deepest weighted branches trade; a weighted parent reports drift and nothing
        // to do about it directly (`.claude/rules/taxonomy-and-rebalance.md`).
        "branches": plan
            .items
            .iter()
            .map(|item| json!({
                "branch": item.label,
                "current": money(item.current_base),
                "current_weight_percent": percent(item.current_weight),
                "target_weight_percent": percent(item.target_weight),
                "drift": money(item.drift_base),
                "trades_here": item.leaf,
                "trades": item.trades
                    .iter()
                    .map(|trade| json!({
                        "instrument": trade.symbol,
                        "quantity": quantity(trade.quantity),
                        "estimated_amount": money(trade.estimated_base),
                        "action": if trade.quantity.is_sign_negative() { "SELL" } else { "BUY" },
                    }))
                    .collect::<Vec<_>>(),
                // A cash subject is paid in, never bought — no price and no quantity.
                "cash_changes": item.deposits
                    .iter()
                    .map(|deposit| json!({
                        "account": deposit.label,
                        "currency": deposit.currency,
                        "amount": money(deposit.amount_base),
                    }))
                    .collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The weights as the card shows them: the branches and their shares, in the order given.
fn weights_text(args: &Value) -> String {
    args.get("weights")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| format!("{} {}%", text(item, "node"), text(item, "percent")))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// The body of `target_save`. A weight is a share of its parent and the store validates each set
/// of siblings, so a target that does not add up is refused here rather than silently reweighing
/// the portfolio (`.claude/rules/taxonomy-and-rebalance.md`).
pub(super) fn rebalance_target_save(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let name = text(args, "target");
    if name.trim().is_empty() {
        return Err(AiError::Tool("a target allocation needs a name".into()));
    }
    let existing = target_by_name(context, &name).ok();

    let mut weights = Vec::new();
    let mut named = Vec::new();
    for item in args
        .get("weights")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let node = node_by_name(context, &taxonomy, &text(item, "node"))?;
        let share = decimal_argument(item, "percent")?
            .ok_or_else(|| AiError::Tool(format!("{} has no weight", node.name)))?;
        named.push(format!("{} {}%", node.name, share.normalize()));
        weights.push(sq_core::model::TargetWeight {
            node_id: node.id,
            weight: share / rust_decimal::Decimal::ONE_HUNDRED,
        });
    }

    let target = sq_core::model::AllocationTarget {
        id: existing
            .as_ref()
            .map(|t| t.id.clone())
            .unwrap_or_else(sq_core::model::new_id),
        portfolio_id: context.scope.portfolio.id.clone(),
        taxonomy_id: taxonomy.id.clone(),
        name: name.trim().to_string(),
        weights,
    };
    target.validate().map_err(tool)?;
    context.store.save_target(&target).map_err(tool)?;
    (context.changed)("targets");

    Ok(json!({
        "target": target.name,
        "tree": taxonomy.name,
        "created": existing.is_none(),
        "weights": named,
    }))
}

pub(super) fn rebalance_target_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let target = target_by_name(context, &text(args, "target"))?;
    context.store.delete_target(&target.id).map_err(tool)?;
    (context.changed)("targets");
    Ok(json!({ "removed": target.name }))
}
