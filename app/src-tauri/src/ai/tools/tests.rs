use super::*;
use serde_json::json;

#[test]
fn every_tool_has_a_strict_object_schema_and_a_unique_name() {
    let mut seen = std::collections::BTreeSet::new();
    for tool in CATALOGUE {
        assert!(seen.insert(tool.name), "duplicate tool {}", tool.name);
        let schema = (tool.schema)();
        assert_eq!(schema["type"], "object", "{}", tool.name);
        // `strict: true` requires every property to be listed as required — an optional
        // argument is spelled as a nullable type instead.
        let properties = schema["properties"].as_object().expect("properties");
        let required: Vec<&str> = schema["required"]
            .as_array()
            .expect("required")
            .iter()
            .map(|v| v.as_str().expect("a name"))
            .collect();
        assert_eq!(properties.len(), required.len(), "{}", tool.name);
        for name in properties.keys() {
            assert!(required.contains(&name.as_str()), "{}: {name}", tool.name);
        }
        assert_eq!(schema["additionalProperties"], false, "{}", tool.name);
    }
}

/// The card the user presses is written in the frontend, so a tool added here without a
/// label there would ask for permission under its own wire name. Read back from the source
/// the same way `guide::SCREEN_IDS` is read back from `nav.tsx`.
#[test]
fn every_tool_the_user_is_asked_about_is_named_in_the_frontend() {
    let source =
        std::fs::read_to_string("../src/components/domain/aiToolLabels.ts").expect("aiToolLabels.ts");
    let (_, after) = source
        .split_once("export const TOOL_LABELS")
        .expect("TOOL_LABELS");
    let (table, _) = after.split_once("\n};").expect("the table ends");

    for tool in CATALOGUE.iter().filter(|t| t.access != Access::Free) {
        assert!(
            table.contains(&format!("{}: msg`", tool.name)),
            "{} has no label in aiToolLabels.ts",
            tool.name
        );
    }
}

#[test]
fn the_catalogue_is_reachable_by_name() {
    assert!(find("portfolio_overview").is_some());
    assert!(find("no_such_tool").is_none());
}

fn empty_context() -> (Store, ScopeSelection) {
    let store = Store::open_in_memory().unwrap();
    let portfolio = sq_core::model::Portfolio::new("Test", "EUR");
    store.save_portfolio(&portfolio).unwrap();
    let scope = ScopeSelection {
        portfolio,
        accounts: Vec::new(),
    };
    (store, scope)
}

/// Arguments satisfying a schema without naming anything: every nullable property is null and
/// a period is the shipped id. A tool that also needs a name — an instrument, a tree, a list —
/// cannot be answered by an empty portfolio and is covered by the test below instead.
fn args_without_names(schema: &Value) -> Option<Value> {
    let properties = schema["properties"].as_object()?;
    let mut args = serde_json::Map::new();
    for (name, property) in properties {
        let types = match &property["type"] {
            Value::String(one) => vec![one.as_str()],
            Value::Array(many) => many.iter().filter_map(Value::as_str).collect(),
            _ => return None,
        };
        if types.contains(&"null") {
            args.insert(name.clone(), Value::Null);
        } else if property.get("enum").is_some() && name == "period" {
            args.insert(name.clone(), json!("ONE_YEAR"));
        } else {
            return None;
        }
    }
    Some(Value::Object(args))
}

/// A portfolio with nothing in it is the shape every tool meets first — a fresh install, or
/// an account picker narrowed to an empty account. "Nothing yet" is an answer; an error is
/// not, and neither is a panic on an empty list.
#[test]
fn every_tool_that_needs_no_name_answers_an_empty_portfolio() {
    let (store, scope) = empty_context();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };

    let mut exercised = 0;
    for tool in CATALOGUE.iter().filter(|t| t.access != Access::Write) {
        let Some(args) = args_without_names(&(tool.schema)()) else {
            continue;
        };
        exercised += 1;
        let answer = (tool.run)(&context, &args);
        assert!(
            answer.is_ok(),
            "{} failed on an empty portfolio: {answer:?}",
            tool.name
        );
        // The card is built from the same arguments, and it is drawn before the tool runs.
        let _ = (tool.summary)(&context, &args);
    }
    assert!(exercised >= 10, "only {exercised} tools were exercised");
}

/// A name the portfolio does not have is the model's mistake to correct, so it comes back as
/// a tool error naming what was not found — not as a failed turn and not as an empty answer
/// that reads like "you have no watchlist".
#[test]
fn a_name_that_does_not_exist_is_reported_back_to_the_model() {
    let (store, scope) = empty_context();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };

    let cases = [
        ("watchlist_rows", json!({ "list": "Nope", "period": "ONE_YEAR" })),
        ("allocation_members", json!({ "tree": "Nope", "node": null })),
        (
            "rebalance_plan",
            json!({ "target": "Nope", "cash_to_invest": null, "allow_sell": null }),
        ),
        ("securities_events", json!({ "symbol": "NOPE", "limit": null })),
        ("market_quotes", json!({ "symbol": "NOPE", "period": "ONE_YEAR" })),
    ];
    for (name, args) in cases {
        let tool = find(name).expect(name);
        match (tool.run)(&context, &args) {
            Err(AiError::Tool(message)) => {
                assert!(
                    message.to_lowercase().contains("nope"),
                    "{name}: {message} does not say what was not found"
                );
            }
            other => panic!("{name} answered {other:?} instead of naming the miss"),
        }
    }
}

/// A portfolio with one depot, its deposit account and one instrument — enough for every
/// write tool to have something real to name.
fn stocked() -> (Store, ScopeSelection) {
    let store = Store::open_in_memory().unwrap();
    let cash = sq_core::model::Account::deposit("Cash EUR", "EUR");
    let depot = sq_core::model::Account::securities("Depot", "EUR", &cash.id);
    store.save_account(&cash).unwrap();
    store.save_account(&depot).unwrap();
    let security = sq_core::model::Security::new(
        "IWDA.L",
        "iShares Core MSCI World",
        "USD",
        sq_core::model::SecurityKind::Etf,
    );
    store.save_security(&security).unwrap();

    let mut portfolio = sq_core::model::Portfolio::new("Test", "EUR");
    portfolio.account_ids = vec![cash.id.clone(), depot.id.clone()];
    store.save_portfolio(&portfolio).unwrap();
    let scope = ScopeSelection {
        portfolio,
        accounts: vec![cash.id, depot.id],
    };
    (store, scope)
}

/// Records every scope a tool announced, so a write that nobody is told about fails here
/// rather than showing up as a screen that will not refresh.
#[derive(Default)]
struct Announced(std::cell::RefCell<Vec<&'static str>>);

impl Announced {
    fn seen(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

#[test]
fn a_purchase_is_written_the_way_the_editor_writes_one_and_is_announced() {
    let (store, scope) = stocked();
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let args = json!({
        "kind": "BUY",
        "account": "depot",
        "date": "2026-09-14",
        "symbol": "iwda.l",
        "quantity": "10",
        "price": "95.5",
        "amount": null,
        "fees": "1.20",
        "taxes": null,
        "currency": null,
        "note": null,
    });
    let answer = (find("transaction_create").unwrap().run)(&context, &args).unwrap();

    // 10 × 95.5 = 955: the amount of a share-moving row is its own legs multiplied, never a
    // number typed beside them.
    assert_eq!(answer["amount"], "955");
    assert_eq!(answer["currency"], "EUR", "the account's own currency");
    assert_eq!(announced.seen(), vec!["transactions"]);

    let depot = store
        .list_accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Depot")
        .unwrap();
    let written = store.transactions_for_account(&depot.id).unwrap();
    let saved = written.first().expect("one row");
    assert_eq!(saved.kind, sq_core::model::TransactionKind::Buy);
    assert_eq!(saved.date.to_string(), "2026-09-14");
    assert_eq!(saved.fees.to_string(), "1.20");
}

#[test]
fn an_operation_that_moves_shares_without_an_instrument_is_refused() {
    let (store, scope) = stocked();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };
    let args = json!({
        "kind": "BUY",
        "account": "Depot",
        "date": null,
        "symbol": null,
        "quantity": "10",
        "price": "95.5",
        "amount": null,
        "fees": null,
        "taxes": null,
        "currency": null,
        "note": null,
    });

    assert!(matches!(
        (find("transaction_create").unwrap().run)(&context, &args),
        Err(AiError::Tool(_))
    ));
}

#[test]
fn a_watchlist_is_replaced_by_what_the_call_names_rather_than_added_to() {
    let (store, scope) = stocked();
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let args = json!({ "list": "Core", "symbols": ["IWDA.L", "IWDA.L"] });
    let answer = (find("watchlist_set").unwrap().run)(&context, &args).unwrap();
    assert_eq!(answer["created"], true);
    // The same ticker twice is one instrument, not a list of two.
    assert_eq!(answer["instruments"].as_array().unwrap().len(), 1);
    assert_eq!(announced.seen(), vec!["watchlists"]);

    let lists = store.list_watchlists().unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].security_ids.len(), 1);

    // Saving the same name again edits that list rather than making a second one.
    let emptied = json!({ "list": "core", "symbols": [] });
    let answer = (find("watchlist_set").unwrap().run)(&context, &emptied).unwrap();
    assert_eq!(answer["created"], false);
    assert_eq!(store.list_watchlists().unwrap().len(), 1);
}

#[test]
fn an_instrument_is_filed_under_a_branch_and_a_share_over_all_of_it_is_refused() {
    let (store, scope) = stocked();
    // A name of its own: three trees are seeded with the database, and "Region" is one of
    // them — the tool would find the seeded one first.
    let taxonomy = sq_core::model::Taxonomy::new("Buckets", sq_core::model::TaxonomyKind::Custom);
    store.save_taxonomy(&taxonomy).unwrap();
    let node = sq_core::model::TaxonomyNode::root(&taxonomy.id, "Developed markets");
    store.save_taxonomy_node(&node).unwrap();

    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let args = json!({
        "tree": "buckets",
        "node": "developed markets",
        "symbol": "IWDA.L",
        "share_percent": "60",
    });
    let answer = (find("taxonomy_assign").unwrap().run)(&context, &args).unwrap();
    assert_eq!(answer["share_percent"], "60");
    assert_eq!(announced.seen(), vec!["taxonomies"]);

    // More than all of an instrument would silently change every weight in the tree.
    let impossible = json!({
        "tree": "Buckets",
        "node": "Developed markets",
        "symbol": "IWDA.L",
        "share_percent": "140",
    });
    assert!(matches!(
        (find("taxonomy_assign").unwrap().run)(&context, &impossible),
        Err(AiError::Tool(_))
    ));
}

#[test]
fn a_rule_can_be_removed_but_only_when_the_call_says_which_one() {
    let (store, scope) = stocked();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };

    let created = json!({
        "symbol": "IWDA.L",
        "kind": "PRICE",
        "price": "100",
        "direction": "UP",
        "date": null,
        "note": null,
    });
    (find("alert_create").unwrap().run)(&context, &created).unwrap();
    assert_eq!(store.list_alerts().unwrap().len(), 1);

    // Neither a level nor a date: the call has not said which rule, and guessing would
    // delete the wrong one.
    let vague = json!({ "symbol": "IWDA.L", "price": null, "date": null });
    assert!(matches!(
        (find("alert_delete").unwrap().run)(&context, &vague),
        Err(AiError::Tool(_))
    ));

    let named = json!({ "symbol": "IWDA.L", "price": "100", "date": null });
    let answer = (find("alert_delete").unwrap().run)(&context, &named).unwrap();
    assert_eq!(answer["removed"], 1);
    assert!(store.list_alerts().unwrap().is_empty());
}

/// The card is what the user presses, so it has to carry the *values* of the change — the
/// difference between "the assistant wants to record a purchase" and "10 at 95.5".
#[test]
fn a_write_card_carries_the_figures_of_the_change() {
    let (store, scope) = stocked();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };
    let args = json!({
        "kind": "BUY",
        "account": "Depot",
        "date": null,
        "symbol": "IWDA.L",
        "quantity": "10",
        "price": "95.5",
        "amount": null,
        "fees": null,
        "taxes": null,
        "currency": null,
        "note": null,
    });

    let card = (find("transaction_create").unwrap().summary)(&context, &args);
    assert_eq!(card.get("quantity").map(String::as_str), Some("10"));
    assert_eq!(card.get("price").map(String::as_str), Some("95.5"));
    assert_eq!(card.get("symbol").map(String::as_str), Some("IWDA.L"));
    // An argument the call left out is left off the card rather than shown as blank.
    assert!(!card.contains_key("amount"));
    // A date the model did not give is resolved before the user is asked, not after.
    assert_eq!(card.get("date").map(String::as_str), Some("2026-09-15"));
}

#[test]
fn a_write_tool_shows_the_change_on_its_card_not_merely_its_name() {
    // A card for a write has to say what will happen, so its summary is never empty — the
    // difference between "the assistant wants to edit an instrument" and naming which one.
    let store = Store::open_in_memory().unwrap();
    let portfolio = sq_core::model::Portfolio::new("Test", "EUR");
    store.save_portfolio(&portfolio).unwrap();
    let scope = ScopeSelection {
        portfolio,
        accounts: Vec::new(),
    };
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };

    for tool in CATALOGUE.iter().filter(|t| t.access == Access::Write) {
        let args = json!({ "symbol": "IWDA.L", "price": "100", "direction": "UP" });
        assert!(
            !(tool.summary)(&context, &args).is_empty(),
            "{} asks for a change without naming it",
            tool.name
        );
    }
}

#[test]
fn only_tools_that_read_no_portfolio_data_skip_the_consent_card() {
    for tool in CATALOGUE {
        if tool.access == Access::Free {
            assert!(
                tool.name.starts_with("app_"),
                "{} reads the portfolio, so it must ask",
                tool.name
            );
        }
    }
}

/// A row is named by what the user would say about it — a date, an operation, an instrument —
/// so the matching has to be exact about *which* row before anything is changed or deleted.
#[test]
fn an_edit_names_one_row_and_refuses_when_two_answer_to_the_same_description() {
    let (store, scope) = stocked();
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let buy = json!({
        "kind": "BUY", "account": "Depot", "date": "2026-09-14", "symbol": "IWDA.L",
        "quantity": "10", "price": "95.5", "amount": null, "fees": null, "taxes": null,
        "currency": null, "note": null,
    });
    (find("transaction_create").unwrap().run)(&context, &buy).unwrap();
    let deposit = json!({
        "kind": "DEPOSIT", "account": "Cash EUR", "date": "2026-09-14", "symbol": null,
        "quantity": null, "price": null, "amount": "500", "fees": null, "taxes": null,
        "currency": null, "note": null,
    });
    (find("transaction_create").unwrap().run)(&context, &deposit).unwrap();

    // Two operations share the day, so the date alone does not say which one is meant.
    let vague = json!({
        "date": "2026-09-14", "kind": null, "symbol": null, "account": null,
        "new_date": null, "new_quantity": "12", "new_price": null, "new_amount": null,
        "new_fees": null, "new_taxes": null, "new_note": null,
    });
    assert!(matches!(
        (find("transaction_update").unwrap().run)(&context, &vague),
        Err(AiError::Tool(_))
    ));

    let named = json!({
        "date": "2026-09-14", "kind": "BUY", "symbol": null, "account": null,
        "new_date": null, "new_quantity": "12", "new_price": null, "new_amount": null,
        "new_fees": "2", "new_taxes": null, "new_note": null,
    });
    let answer = (find("transaction_update").unwrap().run)(&context, &named).unwrap();
    // 12 × 95.5 = 1146: the amount follows the legs, it is not carried over from before.
    assert_eq!(answer["amount"], "1146");
    assert_eq!(answer["fees"], "2");

    let removed = json!({ "date": "2026-09-14", "kind": "DEPOSIT", "symbol": null, "account": null });
    let answer = (find("transaction_delete").unwrap().run)(&context, &removed).unwrap();
    assert_eq!(answer["amount"], "500");

    let depot = store
        .list_accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Depot")
        .unwrap();
    assert_eq!(store.transactions_for_account(&depot.id).unwrap().len(), 1);
}

#[test]
fn an_account_joins_the_portfolio_as_it_is_opened_and_a_depot_needs_somewhere_to_settle() {
    let (store, scope) = stocked();
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let orphan = json!({
        "name": "Depot 2", "currency": "EUR", "kind": "SECURITIES",
        "settlement_account": null, "opened_at": null,
    });
    assert!(matches!(
        (find("account_create").unwrap().run)(&context, &orphan),
        Err(AiError::Tool(_))
    ));

    let cash = json!({
        "name": "Cash USD", "currency": "usd", "kind": "DEPOSIT",
        "settlement_account": null, "opened_at": "2026-01-02",
    });
    let answer = (find("account_create").unwrap().run)(&context, &cash).unwrap();
    assert_eq!(answer["currency"], "USD");
    assert_eq!(announced.seen(), vec!["accounts"]);

    // An account outside the portfolio is one no report would count, so it is in it already.
    let portfolio = store.list_portfolios().unwrap().remove(0);
    assert_eq!(portfolio.account_ids.len(), 3);

    // The same name twice would leave every later call guessing which one was meant.
    assert!(matches!(
        (find("account_create").unwrap().run)(&context, &cash),
        Err(AiError::Tool(_))
    ));
}

#[test]
fn what_the_ledger_holds_is_not_deleted_by_a_sentence() {
    let (store, scope) = stocked();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };
    let buy = json!({
        "kind": "BUY", "account": "Depot", "date": "2026-09-14", "symbol": "IWDA.L",
        "quantity": "10", "price": "95.5", "amount": null, "fees": null, "taxes": null,
        "currency": null, "note": null,
    });
    (find("transaction_create").unwrap().run)(&context, &buy).unwrap();

    // An instrument that has been traded is held by its own operations.
    assert!(matches!(
        (find("security_delete").unwrap().run)(&context, &json!({ "symbol": "IWDA.L" })),
        Err(AiError::Tool(_))
    ));
    // And an account is not emptied silently: taking its operations has to be said out loud.
    let quiet = json!({ "account": "Depot", "with_transactions": null });
    assert!(matches!(
        (find("account_delete").unwrap().run)(&context, &quiet),
        Err(AiError::Tool(_))
    ));

    let asked = json!({ "account": "Depot", "with_transactions": true });
    let answer = (find("account_delete").unwrap().run)(&context, &asked).unwrap();
    assert_eq!(answer["transactions_removed"], 1);
    assert_eq!(store.list_portfolios().unwrap()[0].account_ids.len(), 1);
}

#[test]
fn a_target_that_does_not_add_up_is_refused_before_it_reweighs_anything() {
    let (store, scope) = stocked();
    let taxonomy = sq_core::model::Taxonomy::new("Buckets", sq_core::model::TaxonomyKind::Custom);
    store.save_taxonomy(&taxonomy).unwrap();
    for name in ["Shares", "Bonds"] {
        store
            .save_taxonomy_node(&sq_core::model::TaxonomyNode::root(&taxonomy.id, name))
            .unwrap();
    }
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let lopsided = json!({
        "target": "Mine", "tree": "Buckets",
        "weights": [{ "node": "Shares", "percent": "60" }, { "node": "Bonds", "percent": "60" }],
    });
    assert!(matches!(
        (find("rebalance_target_save").unwrap().run)(&context, &lopsided),
        Err(AiError::Tool(_))
    ));
    assert!(
        store
            .targets_for_portfolio(&scope.portfolio.id)
            .unwrap()
            .is_empty()
    );

    let sound = json!({
        "target": "Mine", "tree": "buckets",
        "weights": [{ "node": "shares", "percent": "60" }, { "node": "Bonds", "percent": "40" }],
    });
    let answer = (find("rebalance_target_save").unwrap().run)(&context, &sound).unwrap();
    assert_eq!(answer["created"], true);
    assert_eq!(announced.seen(), vec!["targets"]);
    let saved = store.targets_for_portfolio(&scope.portfolio.id).unwrap();
    assert_eq!(saved.len(), 1);
    // Stored as a fraction of the parent, whatever the model wrote it as.
    let mut shares: Vec<String> = saved[0]
        .weights
        .iter()
        .map(|w| w.weight.normalize().to_string())
        .collect();
    shares.sort();
    assert_eq!(shares, vec!["0.4".to_string(), "0.6".to_string()]);

    // Saving under the same name edits that target rather than making a second one.
    let answer = (find("rebalance_target_save").unwrap().run)(&context, &sound).unwrap();
    assert_eq!(answer["created"], false);
    assert_eq!(store.targets_for_portfolio(&scope.portfolio.id).unwrap().len(), 1);
}

/// Excluding freezes rather than deletes: the filing stays exactly where it was, so switching
/// the subject back on restores the tree instead of asking the user to file it again.
#[test]
fn excluding_a_subject_leaves_its_filing_untouched() {
    let (store, scope) = stocked();
    let taxonomy = sq_core::model::Taxonomy::new("Buckets", sq_core::model::TaxonomyKind::Custom);
    store.save_taxonomy(&taxonomy).unwrap();
    let node = sq_core::model::TaxonomyNode::root(&taxonomy.id, "Shares");
    store.save_taxonomy_node(&node).unwrap();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };

    let filed = json!({
        "tree": "Buckets", "node": "Shares", "symbol": "IWDA.L", "share_percent": null,
    });
    (find("taxonomy_assign").unwrap().run)(&context, &filed).unwrap();

    let off = json!({
        "tree": "Buckets", "symbol": "IWDA.L", "account": null, "currency": null, "excluded": true,
    });
    let answer = (find("taxonomy_exclude").unwrap().run)(&context, &off).unwrap();
    assert_eq!(answer["excluded"], true);
    assert_eq!(store.taxonomy_exclusions(&taxonomy.id).unwrap().len(), 1);
    assert_eq!(store.classifications_for_taxonomy(&taxonomy.id).unwrap().len(), 1);

    // Taking it out of the branch is the other thing, and it is a different tool.
    let out = json!({ "tree": "Buckets", "node": "Shares", "symbol": "IWDA.L" });
    (find("taxonomy_unassign").unwrap().run)(&context, &out).unwrap();
    assert!(
        store
            .classifications_for_taxonomy(&taxonomy.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_plan_is_written_with_its_legs_and_proposes_nothing_by_itself() {
    let (store, scope) = stocked();
    let announced = Announced::default();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|scope| announced.0.borrow_mut().push(scope),
    };

    let saved = json!({
        "plan": "Monthly", "account": "Depot", "amount": "500", "currency": "EUR",
        "every": 1, "unit": "MONTH", "start": "2026-09-01", "end": null,
        "legs": [{ "symbol": "IWDA.L", "percent": "100" }], "fees": "1", "active": null,
    });
    let answer = (find("plan_save").unwrap().run)(&context, &saved).unwrap();
    assert_eq!(answer["created"], true);
    assert_eq!(answer["amount"], "500");
    assert_eq!(announced.seen(), vec!["plans"]);

    // A plan writes nothing on its own: the ledger is still empty until plan_commit.
    let depot = store
        .list_accounts()
        .unwrap()
        .into_iter()
        .find(|a| a.name == "Depot")
        .unwrap();
    assert!(store.transactions_for_account(&depot.id).unwrap().is_empty());

    // The same name edits that plan; a contribution of nothing is not a plan at all.
    let paused = json!({
        "plan": "monthly", "account": null, "amount": null, "currency": null,
        "every": null, "unit": null, "start": null, "end": null, "legs": null,
        "fees": null, "active": false,
    });
    let answer = (find("plan_save").unwrap().run)(&context, &paused).unwrap();
    assert_eq!(answer["created"], false);
    assert_eq!(answer["active"], false);
    assert_eq!(store.list_plans(&scope.portfolio.id).unwrap().len(), 1);

    let impossible = json!({
        "plan": "Monthly", "account": null, "amount": "0", "currency": null,
        "every": null, "unit": null, "start": null, "end": null, "legs": null,
        "fees": null, "active": null,
    });
    assert!(matches!(
        (find("plan_save").unwrap().run)(&context, &impossible),
        Err(AiError::Tool(_))
    ));
}

/// The model's one line on why is asked for in the same call it explains — a reason that
/// travelled separately could be about a different call by the time the card is drawn.
#[test]
fn every_tool_the_user_is_asked_about_also_asks_the_model_why() {
    for (name, _, schema) in definitions() {
        let asks = find(name).expect(name).access != Access::Free;
        let declared = schema["properties"].get(REASON).is_some();
        let required = schema["required"]
            .as_array()
            .expect("required")
            .iter()
            .any(|v| v.as_str() == Some(REASON));

        assert_eq!(
            declared, asks,
            "{name}: reason declared without a card, or missing one"
        );
        assert_eq!(
            required, asks,
            "{name}: strict mode needs every property required"
        );
        // Still strict-compatible after the injection: as many required names as properties.
        assert_eq!(
            schema["properties"].as_object().expect("properties").len(),
            schema["required"].as_array().expect("required").len(),
            "{name}"
        );
    }
}

/// It is not an argument: no body reads it, so a call carrying one answers exactly as it would
/// without. Otherwise the reason would have to be stripped in 60 places instead of ignored.
#[test]
fn a_reason_in_the_arguments_changes_no_answer() {
    let (store, scope) = empty_context();
    let context = ToolContext {
        store: &store,
        scope: &scope,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        changed: &|_| {},
    };
    let tool = find("positions_list").expect("positions_list");
    let bare = (tool.run)(&context, &json!({ "limit": null })).unwrap();
    let explained = (tool.run)(
        &context,
        &json!({ "limit": null, "reason": "to answer what they hold" }),
    )
    .unwrap();
    assert_eq!(bare, explained);
}
