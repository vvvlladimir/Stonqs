//! Classification trees: their shape, what is filed where, and the writes that change either.
//!
//! What the tree *divides* — weights, drift, members — lives in `allocation.rs`. Here is the
//! structure itself, so a model asked to file something can see where it would go first.

use super::args::*;
use super::fmt::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const TAXONOMY_TREE: Tool = Tool {
    name: "taxonomy_tree",
    description: "The branches of one classification tree and how many instruments sit in \
                  each. Call it before filing anything: a branch has to exist before an \
                  instrument can go into it, and the names here are the ones to use.",
    access: Access::Ask,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name, as allocation_trees returned it." }
            },
            "required": ["tree"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("tree", text(args, "tree")),
    run: taxonomy_tree,
};

pub(super) const TAXONOMY_ASSIGN: Tool = Tool {
    name: "taxonomy_assign",
    description: "File an instrument under one branch of a classification tree, or change \
                  the share of it that counts there. An instrument may sit in several \
                  branches of the same tree at different shares.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name, as allocation_trees returned it." },
                "node": { "type": "string", "description": "The branch's name, as taxonomy_tree returned it." },
                "symbol": { "type": "string", "description": "The instrument's ticker." },
                "share_percent": { "type": ["string", "null"], "description": "How much of the instrument counts under this branch, 0 to 100. Null means all of it." }
            },
            "required": ["tree", "node", "symbol", "share_percent"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("tree".into(), text(args, "tree"));
        params.insert("node".into(), text(args, "node"));
        params.insert(
            "share_percent".into(),
            optional(args, "share_percent").unwrap_or_else(|| "100".into()),
        );
        params
    },
    run: taxonomy_assign,
};

pub(super) const TAXONOMY_UNASSIGN: Tool = Tool {
    name: "taxonomy_unassign",
    description: "Take an instrument out of one branch of a classification tree. What is left \
                  unclassified still counts in the portfolio — it just falls into the \
                  unclassified share of that tree.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name." },
                "node": { "type": "string", "description": "The branch the instrument is filed under." },
                "symbol": { "type": "string", "description": "The instrument's ticker." }
            },
            "required": ["tree", "node", "symbol"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("instrument".into(), text(args, "symbol"));
        params.insert("tree".into(), text(args, "tree"));
        params.insert("node".into(), text(args, "node"));
        params
    },
    run: taxonomy_unassign,
};

pub(super) const TAXONOMY_EXCLUDE: Tool = Tool {
    name: "taxonomy_exclude",
    description: "Switch one instrument or one account balance off for a single classification \
                  tree, or switch it back on. An excluded subject leaves that tree's \
                  percentages entirely instead of falling into its unclassified share, and its \
                  filing is kept untouched for when it comes back.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree this applies to. Other trees are unaffected." },
                "symbol": { "type": ["string", "null"], "description": "The instrument's ticker. Null when excluding an account balance." },
                "account": { "type": ["string", "null"], "description": "The account holding the cash, by name. Null when excluding an instrument." },
                "currency": { "type": ["string", "null"], "description": "Which currency of that account. One account holding two currencies is two subjects." },
                "excluded": { "type": "boolean", "description": "True switches the subject off for this tree, false brings it back." }
            },
            "required": ["tree", "symbol", "account", "currency", "excluded"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("tree".into(), text(args, "tree"));
        for key in ["symbol", "account", "currency"] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        params.insert(
            "excluded".into(),
            args.get("excluded")
                .and_then(Value::as_bool)
                .unwrap_or(true)
                .to_string(),
        );
        params
    },
    run: taxonomy_exclude,
};

pub(super) const TAXONOMY_CREATE: Tool = Tool {
    name: "taxonomy_create",
    description: "Start a new, empty classification tree. Add its branches with \
                  taxonomy_node_add and then file instruments into them.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "What to call the tree, in the user's own words." }
            },
            "required": ["name"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("name", text(args, "name")),
    run: taxonomy_create,
};

pub(super) const TAXONOMY_DELETE: Tool = Tool {
    name: "taxonomy_delete",
    description: "Delete a whole classification tree: its branches, everything filed into \
                  them and any target allocation written against it. The instruments \
                  themselves are untouched.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name." }
            },
            "required": ["tree"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("tree", text(args, "tree")),
    run: taxonomy_delete,
};

pub(super) const TAXONOMY_NODE_ADD: Tool = Tool {
    name: "taxonomy_node_add",
    description: "Add one branch to a classification tree, at the top or under an existing \
                  branch.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name." },
                "name": { "type": "string", "description": "What to call the branch." },
                "parent": { "type": ["string", "null"], "description": "The branch it sits under. Null puts it at the top level." }
            },
            "required": ["tree", "name", "parent"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("tree".into(), text(args, "tree"));
        params.insert("name".into(), text(args, "name"));
        params.insert("parent".into(), or_all(args, "parent"));
        params
    },
    run: taxonomy_node_add,
};

pub(super) const TAXONOMY_NODE_DELETE: Tool = Tool {
    name: "taxonomy_node_delete",
    description: "Remove one branch from a classification tree, along with the branches under \
                  it and everything filed into them. The instruments themselves stay.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "tree": { "type": "string", "description": "The tree's name." },
                "node": { "type": "string", "description": "The branch to remove." }
            },
            "required": ["tree", "node"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("tree".into(), text(args, "tree"));
        params.insert("node".into(), text(args, "node"));
        params
    },
    run: taxonomy_node_delete,
};

pub(super) fn taxonomy_tree(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let nodes = context.store.taxonomy_nodes(&taxonomy.id).map_err(tool)?;
    let filed = context
        .store
        .classifications_for_taxonomy(&taxonomy.id)
        .map_err(tool)?;
    let cash = context
        .store
        .cash_classifications_for_taxonomy(&taxonomy.id)
        .map_err(tool)?;

    Ok(json!({
        "tree": taxonomy.name,
        "kind": format!("{:?}", taxonomy.kind),
        "branches": nodes
            .iter()
            .map(|node| json!({
                "name": node.name,
                "parent": node
                    .parent_id
                    .as_ref()
                    .and_then(|id| nodes.iter().find(|n| &n.id == id))
                    .map(|n| n.name.as_str()),
                "instruments": filed.iter().filter(|c| c.node_id == node.id).count(),
                "cash_balances": cash.iter().filter(|c| c.node_id == node.id).count(),
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The body of `classification_save` for an instrument. A share is stored as a fraction, and the
/// store refuses one outside (0, 1] — an instrument counted twice over would silently change
/// every weight in the tree (`.claude/rules/taxonomy-and-rebalance.md`).
pub(super) fn taxonomy_assign(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let node = node_by_name(context, &taxonomy, &text(args, "node"))?;
    let security = security_by_symbol(context, &text(args, "symbol"))?;

    let share = decimal_argument(args, "share_percent")?.unwrap_or(rust_decimal::Decimal::ONE_HUNDRED);
    let weight = share / rust_decimal::Decimal::ONE_HUNDRED;
    let assignment = sq_core::model::SecurityClassification::new(&security.id, &node.id, weight);
    assignment.validate().map_err(tool)?;
    context.store.save_classification(&assignment).map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({
        "instrument": security.symbol,
        "tree": taxonomy.name,
        "node": node.name,
        "share_percent": percent(weight),
    }))
}

pub(super) fn taxonomy_unassign(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let node = node_by_name(context, &taxonomy, &text(args, "node"))?;
    let security = security_by_symbol(context, &text(args, "symbol"))?;

    context
        .store
        .delete_classification(&security.id, &node.id)
        .map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({
        "instrument": security.symbol,
        "tree": taxonomy.name,
        "node": node.name,
        "filed": false,
    }))
}

/// Exclusion is per tree and freezes rather than deletes: the classification stays in place, the
/// subject simply leaves this tree's denominator (`.claude/rules/taxonomy-and-rebalance.md`).
pub(super) fn taxonomy_exclude(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let excluded = args.get("excluded").and_then(Value::as_bool).unwrap_or(true);

    let (subject_id, named) = match (optional(args, "symbol"), optional(args, "account")) {
        (Some(symbol), _) => {
            let security = security_by_symbol(context, &symbol)?;
            (security.id.clone(), security.symbol)
        }
        (None, Some(name)) => {
            let account = account_by_name(context, &name)?;
            // The unit is account *and* currency: one deposit account holding two is two
            // subjects, so the currency is not optional here.
            let currency = optional(args, "currency").unwrap_or_else(|| account.currency.clone());
            (
                sq_core::model::cash_subject_key(&account.id, &currency),
                format!(
                    "{} {}",
                    account.name,
                    sq_core::money::normalize_currency(&currency)
                ),
            )
        }
        (None, None) => {
            return Err(AiError::Tool(
                "say what to exclude: an instrument by ticker, or an account and its currency".into(),
            ));
        }
    };

    context
        .store
        .set_taxonomy_exclusion(&taxonomy.id, &subject_id, excluded)
        .map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({ "tree": taxonomy.name, "subject": named, "excluded": excluded }))
}

pub(super) fn taxonomy_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let name = text(args, "name");
    if name.trim().is_empty() {
        return Err(AiError::Tool("a classification tree needs a name".into()));
    }
    // A tree the assistant makes is the user's own, so it carries no kind of its own: the
    // seeded kinds are labels the shipped trees came with (ADR-0032).
    let taxonomy = sq_core::model::Taxonomy::new(name.trim(), sq_core::model::TaxonomyKind::Custom);
    context.store.save_taxonomy(&taxonomy).map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({ "tree": taxonomy.name, "branches": 0 }))
}

pub(super) fn taxonomy_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let branches = context.store.taxonomy_nodes(&taxonomy.id).map_err(tool)?.len();
    context.store.delete_taxonomy(&taxonomy.id).map_err(tool)?;
    (context.changed)("taxonomies");
    // A target is written against a tree, so it goes with it; the frontend hears about both.
    (context.changed)("targets");

    Ok(json!({ "removed": taxonomy.name, "branches": branches }))
}

pub(super) fn taxonomy_node_add(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let name = text(args, "name");
    if name.trim().is_empty() {
        return Err(AiError::Tool("a branch needs a name".into()));
    }
    let parent = match optional(args, "parent") {
        Some(wanted) => Some(node_by_name(context, &taxonomy, &wanted)?),
        None => None,
    };

    let node = match &parent {
        Some(parent) => sq_core::model::TaxonomyNode::child(parent, name.trim()),
        None => sq_core::model::TaxonomyNode::root(&taxonomy.id, name.trim()),
    };
    context.store.save_taxonomy_node(&node).map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({
        "tree": taxonomy.name,
        "branch": node.name,
        "parent": parent.map(|p| p.name),
    }))
}

pub(super) fn taxonomy_node_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let taxonomy = taxonomy_by_name(context, &text(args, "tree"))?;
    let node = node_by_name(context, &taxonomy, &text(args, "node"))?;
    context.store.delete_taxonomy_node(&node.id).map_err(tool)?;
    (context.changed)("taxonomies");

    Ok(json!({ "tree": taxonomy.name, "removed": node.name }))
}
