//! Accounts and the groups over them. `portfolio.rs` answers what the accounts in view are
//! worth; these are the writes that change which accounts exist at all.
//!
//! A new account joins the portfolio as it is created, exactly as the form does it — an account
//! outside the portfolio is one no report would ever count.

use super::args::*;
use super::lookup::*;
use super::{Access, AiError, AiResult, Params, Tool, ToolContext, tool};
use serde_json::{Value, json};

pub(super) const ACCOUNT_GROUPS_LIST: Tool = Tool {
    name: "account_groups_list",
    description: "The account groups the user keeps and which accounts are in each. A group is \
                  a way of looking at several accounts at once, not an account of its own.",
    access: Access::Ask,
    schema: no_arguments,
    summary: |_, _| Params::new(),
    run: account_groups_list,
};

pub(super) const ACCOUNT_CREATE: Tool = Tool {
    name: "account_create",
    description: "Open an account in the portfolio: a deposit account holding cash, or a \
                  securities account holding instruments. A securities account settles its \
                  money through a deposit account, which has to exist first.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "What to call it, in the user's own words." },
                "currency": { "type": "string", "description": "Its currency, as a three-letter code." },
                "kind": { "type": "string", "enum": ["DEPOSIT", "SECURITIES"], "description": "Cash, or instruments." },
                "settlement_account": { "type": ["string", "null"], "description": "For a securities account: the deposit account its money moves through, by name. Null for a deposit account." },
                "opened_at": { "type": ["string", "null"], "description": "When it was opened, YYYY-MM-DD. Null leaves it unsaid." }
            },
            "required": ["name", "currency", "kind", "settlement_account", "opened_at"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("name".into(), text(args, "name"));
        params.insert("currency".into(), text(args, "currency"));
        params.insert("kind".into(), text(args, "kind"));
        for key in ["settlement_account", "opened_at"] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        params
    },
    run: account_create,
};

pub(super) const ACCOUNT_UPDATE: Tool = Tool {
    name: "account_update",
    description: "Change an account: its name, its currency, the deposit account it settles \
                  through, or whether it is still in use. Everything left null stays as it is.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "account": { "type": "string", "description": "The account's current name." },
                "new_name": { "type": ["string", "null"], "description": "Rename it to this." },
                "currency": { "type": ["string", "null"], "description": "Its currency, as a three-letter code." },
                "settlement_account": { "type": ["string", "null"], "description": "For a securities account: the deposit account its money moves through." },
                "active": { "type": ["boolean", "null"], "description": "False marks it closed without deleting anything." }
            },
            "required": ["account", "new_name", "currency", "settlement_account", "active"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("account".into(), text(args, "account"));
        for key in ["new_name", "currency", "settlement_account"] {
            let value = nullable(args, key);
            if !value.is_empty() {
                params.insert(key.into(), value);
            }
        }
        if let Some(active) = args.get("active").and_then(Value::as_bool) {
            params.insert("active".into(), active.to_string());
        }
        params
    },
    run: account_update,
};

pub(super) const ACCOUNT_DELETE: Tool = Tool {
    name: "account_delete",
    description: "Delete an account. Its transactions go with it, so an account that carries \
                  any is refused unless the call says to take them too. An account other \
                  accounts settle through cannot be deleted at all until they are relinked.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "account": { "type": "string", "description": "The account's name." },
                "with_transactions": { "type": ["boolean", "null"], "description": "True deletes the account together with every operation recorded on it. Null means refuse if it carries any." }
            },
            "required": ["account", "with_transactions"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("account".into(), text(args, "account"));
        params.insert(
            "with_transactions".into(),
            args.get("with_transactions")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .to_string(),
        );
        params
    },
    run: account_delete,
};

pub(super) const ACCOUNT_GROUP_SET: Tool = Tool {
    name: "account_group_set",
    description: "Create an account group or replace which accounts are in one. The accounts \
                  given are the whole group afterwards — read account_groups_list first and \
                  send the existing ones back along with the new.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "group": { "type": "string", "description": "The group's name. A name that does not exist yet creates it." },
                "accounts": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Every account the group should hold afterwards, by name."
                }
            },
            "required": ["group", "accounts"],
            "additionalProperties": false
        })
    },
    summary: |_, args| {
        let mut params = Params::new();
        params.insert("group".into(), text(args, "group"));
        params.insert("accounts".into(), names_of(args, "accounts").join(", "));
        params
    },
    run: account_group_set,
};

pub(super) const ACCOUNT_GROUP_DELETE: Tool = Tool {
    name: "account_group_delete",
    description: "Delete an account group. The accounts in it are untouched — a group is only \
                  a way of looking at them.",
    access: Access::Write,
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "group": { "type": "string", "description": "The group's name." }
            },
            "required": ["group"],
            "additionalProperties": false
        })
    },
    summary: |_, args| one("group", text(args, "group")),
    run: account_group_delete,
};

pub(super) fn account_groups_list(context: &ToolContext, _args: &Value) -> AiResult<Value> {
    let accounts = context.store.list_accounts().map_err(tool)?;
    let groups: Vec<Value> = context
        .store
        .list_account_groups()
        .map_err(tool)?
        .iter()
        .map(|group| {
            json!({
                "name": group.name,
                "accounts": group
                    .account_ids
                    .iter()
                    .filter_map(|id| accounts.iter().find(|a| &a.id == id))
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({ "groups": groups }))
}

pub(super) fn account_create(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let name = text(args, "name").trim().to_string();
    if name.is_empty() {
        return Err(AiError::Tool("an account needs a name".into()));
    }
    if account_by_name(context, &name).is_ok() {
        return Err(AiError::Tool(format!(
            "there is already an account called {name}"
        )));
    }
    let currency = sq_core::money::normalize_currency(&text(args, "currency"));
    if currency.len() != 3 {
        return Err(AiError::Tool(format!(
            "{currency:?} is not a three-letter currency code"
        )));
    }

    let account = match text(args, "kind").as_str() {
        "SECURITIES" => {
            // A depot without a settlement account cannot record a purchase, so the reference
            // is required at creation rather than set later.
            let settles = optional(args, "settlement_account").ok_or_else(|| {
                AiError::Tool("a securities account needs the deposit account it settles through".into())
            })?;
            let reference = account_by_name(context, &settles)?;
            if reference.kind != sq_core::model::AccountKind::Deposit {
                return Err(AiError::Tool(format!(
                    "{} is not a deposit account, so nothing can settle through it",
                    reference.name
                )));
            }
            sq_core::model::Account::securities(&name, &currency, &reference.id)
        }
        _ => sq_core::model::Account::deposit(&name, &currency),
    };
    let account = sq_core::model::Account {
        opened_at: date_argument(args, "opened_at")?,
        ..account
    };
    context.store.save_account(&account).map_err(tool)?;

    // An account outside the portfolio is one no report counts, so it joins as it is created.
    let mut portfolio = context.scope.portfolio.clone();
    portfolio.account_ids.push(account.id.clone());
    context.store.save_portfolio(&portfolio).map_err(tool)?;
    (context.changed)("accounts");

    Ok(json!({
        "account": account.name,
        "kind": format!("{:?}", account.kind),
        "currency": account.currency,
        "settles_through": account
            .reference_account_id
            .as_ref()
            .and_then(|id| context.store.get_account(id).ok())
            .map(|a| a.name),
    }))
}

pub(super) fn account_update(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let existing = account_by_name(context, &text(args, "account"))?;

    let reference = match optional(args, "settlement_account") {
        Some(name) => Some(account_by_name(context, &name)?),
        None => None,
    };
    let updated = sq_core::model::Account {
        name: optional(args, "new_name").unwrap_or_else(|| existing.name.clone()),
        currency: optional(args, "currency")
            .map(|c| sq_core::money::normalize_currency(&c))
            .unwrap_or_else(|| existing.currency.clone()),
        reference_account_id: match existing.kind {
            // A deposit account settles itself; naming a reference for one is meaningless.
            sq_core::model::AccountKind::Deposit => None,
            sq_core::model::AccountKind::Securities => reference
                .as_ref()
                .map(|a| a.id.clone())
                .or(existing.reference_account_id.clone()),
        },
        is_active: args
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(existing.is_active),
        ..existing.clone()
    };
    context.store.save_account(&updated).map_err(tool)?;
    (context.changed)("accounts");

    Ok(json!({
        "account": updated.name,
        "was": existing.name,
        "currency": updated.currency,
        "active": updated.is_active,
    }))
}

pub(super) fn account_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let account = account_by_name(context, &text(args, "account"))?;

    let dependents = context.store.accounts_referencing(&account.id).map_err(tool)?;
    if !dependents.is_empty() {
        return Err(AiError::Tool(format!(
            "securities accounts settle through {}: {} — relink them first",
            account.name,
            dependents
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let carried = context
        .store
        .transactions_for_account(&account.id)
        .map_err(tool)?
        .len();
    let with_transactions = args
        .get("with_transactions")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if carried > 0 && !with_transactions {
        return Err(AiError::Tool(format!(
            "{} carries {carried} operations, which are deleted with it — say so explicitly to go ahead",
            account.name
        )));
    }

    context.store.delete_account(&account.id).map_err(tool)?;
    let mut portfolio = context.scope.portfolio.clone();
    portfolio.account_ids.retain(|id| id != &account.id);
    context.store.save_portfolio(&portfolio).map_err(tool)?;
    (context.changed)("accounts");

    Ok(json!({ "removed": account.name, "transactions_removed": carried }))
}

pub(super) fn account_group_set(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let name = text(args, "group").trim().to_string();
    if name.is_empty() {
        return Err(AiError::Tool("a group needs a name".into()));
    }
    let existing = group_by_name(context, &name).ok();

    let mut account_ids = Vec::new();
    let mut named = Vec::new();
    for wanted in names_of(args, "accounts") {
        let account = account_by_name(context, &wanted)?;
        if !account_ids.contains(&account.id) {
            account_ids.push(account.id);
            named.push(account.name);
        }
    }
    if account_ids.is_empty() {
        return Err(AiError::Tool("a group needs at least one account".into()));
    }

    let group = sq_core::model::AccountGroup {
        id: existing
            .as_ref()
            .map(|g| g.id.clone())
            .unwrap_or_else(sq_core::model::new_id),
        name,
        account_ids,
    };
    context.store.save_account_group(&group).map_err(tool)?;
    (context.changed)("accounts");

    Ok(json!({ "group": group.name, "created": existing.is_none(), "accounts": named }))
}

pub(super) fn account_group_delete(context: &ToolContext, args: &Value) -> AiResult<Value> {
    let group = group_by_name(context, &text(args, "group"))?;
    context.store.delete_account_group(&group.id).map_err(tool)?;
    (context.changed)("accounts");
    Ok(json!({ "removed": group.name }))
}
