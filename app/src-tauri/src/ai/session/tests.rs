use super::super::fake::StaticAiProvider;
use super::super::{AiError, AiTurn, Usage};
use super::*;
use sq_core::model::Portfolio;
use std::cell::RefCell;

/// Answers every card the same way, and records what it was asked — including the model's
/// stated reason, which is the half of the card the host does not write.
struct ScriptedGate {
    decision: Decision,
    asked: RefCell<Vec<String>>,
    reasons: RefCell<Vec<String>>,
}

impl ScriptedGate {
    fn new(decision: Decision) -> Self {
        ScriptedGate {
            decision,
            asked: RefCell::new(Vec::new()),
            reasons: RefCell::new(Vec::new()),
        }
    }
}

impl ConsentGate for ScriptedGate {
    fn ask(
        &self,
        _request_id: &str,
        tool: &str,
        _params: &tools::Params,
        _write: bool,
        reason: &str,
    ) -> Decision {
        self.asked.borrow_mut().push(tool.to_string());
        self.reasons.borrow_mut().push(reason.to_string());
        self.decision
    }
}

/// An empty portfolio is enough: every assertion here is about the loop, and a tool that
/// answers "nothing held" exercises it exactly as well as one that answers with rows.
fn fixture() -> (Store, String, ScopeSelection) {
    let store = Store::open_in_memory().unwrap();
    let portfolio = Portfolio::new("Test", "EUR");
    store.save_portfolio(&portfolio).unwrap();
    let chat_id = store.ai_chat_create("New chat", "openai", "gpt-5.1").unwrap().id;
    let scope = ScopeSelection {
        portfolio,
        accounts: Vec::new(),
    };
    (store, chat_id, scope)
}

fn session<'a>(
    store: &'a Store,
    chat_id: &'a str,
    scope: &'a ScopeSelection,
    gate: &'a dyn ConsentGate,
    cancelled: &'a dyn Fn() -> bool,
) -> Session<'a> {
    Session {
        store,
        scope,
        quotes_source: None,
        today: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        as_of: None,
        screen: None,
        chat_id,
        web_search: false,
        reasoning_summary: false,
        gate,
        cancelled,
        changed: &|_| {},
        plugin_tools: &[],
    }
}

fn call_turn(tool: &str, args: &str) -> AiTurn {
    AiTurn {
        blocks: vec![Block::ToolCall {
            id: "call_1".into(),
            name: tool.into(),
            args_json: args.into(),
            signature: None,
        }],
        stop_reason: StopReason::ToolUse,
        usage: Usage::default(),
    }
}

#[test]
fn a_plain_turn_is_persisted_on_both_sides_and_replayed_next_time() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::text("hello there");

    let events = RefCell::new(Vec::new());
    send(&session, &provider, "hi".into(), &mut |e| {
        events.borrow_mut().push(e)
    })
    .unwrap();

    assert!(matches!(events.into_inner().last(), Some(AiEvent::Done)));
    assert_eq!(provider.seen.borrow()[0].system, SYSTEM_PROMPT);
    assert!(gate.asked.borrow().is_empty(), "no tool, no card");

    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].role, Role::User);
    assert_eq!(turns[1].role, Role::Model);
}

/// A plugin's tool is offered after the catalogue, asked about like any read, run in its
/// sandbox, and its answer comes back labelled as the plugin's (ADR-0085).
#[test]
fn a_plugin_tool_is_offered_asked_about_and_answers_as_the_plugins() {
    let (store, chat_id, scope) = fixture();
    let dir = std::env::temp_dir().join(format!("stonqs-tool-{}", uuid::Uuid::new_v4()));
    let plugins = crate::plugins::Plugins::new(&dir);
    plugins
        .install(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/concentration"),
        )
        .unwrap();
    let loaded = plugins.tools().unwrap();
    let name = loaded[0].model_name.clone();

    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = Session {
        plugin_tools: &loaded,
        ..session(&store, &chat_id, &scope, &gate, &never)
    };
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn(
            &name,
            r#"{"top": 3, "reason": "to see how spread out you are"}"#,
        )),
        Ok(AiTurn {
            blocks: vec![Block::Text { text: "done".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    let events = RefCell::new(Vec::new());
    send(&session, &provider, "am I diversified?".into(), &mut |e| {
        events.borrow_mut().push(e)
    })
    .unwrap();

    let offered = &provider.seen.borrow()[0].tools;
    let def = offered
        .iter()
        .find(|t| t.name == name)
        .expect("offered to the model");
    assert!(def.description.starts_with("From the Concentration plugin"));
    assert_eq!(
        def.schema["properties"]["reason"]["type"], "string",
        "asked about, so it gives a reason"
    );
    assert_eq!(
        *gate.asked.borrow(),
        std::slice::from_ref(&name),
        "a stranger's code is a read that is asked about"
    );

    let finished = events
        .into_inner()
        .into_iter()
        .find_map(|e| match e {
            AiEvent::ToolFinished { content, .. } => Some(content),
            _ => None,
        })
        .expect("the tool ran");
    // An empty portfolio has no positions; what matters is whose answer it is.
    assert!(finished.contains(r#""plugin":"Concentration""#), "{finished}");
    assert!(finished.contains(r#""answer":{"positions":0}"#), "{finished}");
    std::fs::remove_dir_all(&dir).ok();
}

fn usage(input: i64, output: i64) -> Usage {
    Usage {
        input_tokens: input,
        cached_tokens: 0,
        output_tokens: output,
        reasoning_tokens: 0,
    }
}

#[test]
fn a_turn_counts_every_request_it_took_and_records_each_one() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(AiTurn {
            usage: usage(1_000, 120),
            ..call_turn("app_periods", "{}")
        }),
        Ok(AiTurn {
            blocks: vec![Block::Text {
                text: "here you go".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: usage(1_500, 80),
        }),
    ]);

    let events = RefCell::new(Vec::new());
    send(&session, &provider, "which periods?".into(), &mut |e| {
        events.borrow_mut().push(e)
    })
    .unwrap();

    // The event is the running total of the turn, not of the request: 1000 + 1500 in,
    // 120 + 80 out, and the last one the panel sees is the whole question's cost.
    let last = events
        .into_inner()
        .into_iter()
        .filter_map(|event| match event {
            AiEvent::Usage { usage } => Some(usage),
            _ => None,
        })
        .next_back()
        .expect("the turn reports what it cost");
    assert_eq!(last, usage(2_500, 200));

    // Two requests, two rows: a step is what the provider bills.
    let totals = store.ai_usage_totals().unwrap();
    assert_eq!(totals.len(), 1);
    assert_eq!(totals[0].requests, 2);
    assert_eq!(totals[0].model, "gpt-5.1");
    assert_eq!(store.ai_usage_for_chat(&chat_id).unwrap(), usage(2_500, 200));
}

#[test]
fn a_turn_that_cost_nothing_writes_no_row() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::text("hello");

    send(&session, &provider, "hi".into(), &mut |_| {}).unwrap();

    assert!(store.ai_usage_totals().unwrap().is_empty());
}

#[test]
fn the_model_is_offered_every_tool_in_the_catalogue() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::text("hi");

    send(&session, &provider, "hi".into(), &mut |_| {}).unwrap();

    let offered = &provider.seen.borrow()[0].tools;
    assert_eq!(offered.len(), tools::CATALOGUE.len());
}

#[test]
fn an_allowed_call_runs_and_its_result_goes_back_as_a_user_turn() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn("app_periods", "{}")),
        Ok(AiTurn {
            blocks: vec![Block::Text {
                text: "here you go".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    send(&session, &provider, "which periods?".into(), &mut |_| {}).unwrap();

    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    // user, model(call), user(result), model(text)
    assert_eq!(turns.len(), 4);
    let Block::ToolResult { content, .. } = &turns[2].blocks[0] else {
        panic!("the result is a tool result, not {:?}", turns[2].blocks[0]);
    };
    assert!(
        content.starts_with("<tool_data>"),
        "results are fenced: {content}"
    );
    assert!(content.contains("ONE_MONTH"));
}

#[test]
fn a_free_tool_is_never_put_in_front_of_the_user() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn("app_periods", "{}")),
        Ok(AiTurn {
            blocks: vec![Block::Text { text: "ok".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    send(&session, &provider, "which periods?".into(), &mut |_| {}).unwrap();

    assert!(gate.asked.borrow().is_empty(), "app_* asks nobody");
}

/// The card has two halves and they come from different places: the values are the host's,
/// built from the call's own arguments, and the sentence is the model's. A model that skips
/// the sentence leaves the values standing alone rather than an empty quote.
#[test]
fn the_model_states_why_and_the_host_still_describes_what() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn(
            "positions_list",
            r#"{"limit":null,"reason":"to see what the weight question is about"}"#,
        )),
        Ok(call_turn("accounts_list", "{}")),
        Ok(AiTurn {
            blocks: vec![Block::Text { text: "ok".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    let mut running: Vec<(String, String)> = Vec::new();
    send(
        &session,
        &provider,
        "what is my biggest holding?".into(),
        &mut |event| {
            if let AiEvent::ToolRunning { tool, reason } = event {
                running.push((tool, reason));
            }
        },
    )
    .unwrap();

    assert_eq!(
        gate.reasons.borrow().as_slice(),
        ["to see what the weight question is about", ""],
        "the model's own words, and nothing invented for the call that omitted them"
    );
    // The same sentence rides the running badge, so a call allowed for the whole chat — and
    // therefore never carded — still says what it was for.
    assert_eq!(running[0].1, "to see what the weight question is about");
    // And it is not an argument: the tool answered with it present in the call.
    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    let Block::ToolResult { content, .. } = &turns[2].blocks[0] else {
        panic!("expected a tool result");
    };
    assert!(content.contains("base_currency"), "the reading ran: {content}");
}

#[test]
fn a_refusal_is_answered_to_the_model_rather_than_ending_the_turn() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn("positions_list", r#"{"limit":null}"#)),
        Ok(AiTurn {
            blocks: vec![Block::Text {
                text: "I cannot see that".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    send(&session, &provider, "what do I hold?".into(), &mut |_| {}).unwrap();

    assert_eq!(gate.asked.borrow().as_slice(), ["positions_list"]);
    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    let Block::ToolResult { content, .. } = &turns[2].blocks[0] else {
        panic!("expected a tool result");
    };
    assert!(content.contains("did not allow"));
    // The conversation still ended in an answer.
    assert!(matches!(turns[3].blocks[0], Block::Text { .. }));
}

#[test]
fn allowing_for_the_chat_is_remembered_and_not_asked_twice() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Session);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let calls = || {
        StaticAiProvider::script(vec![
            Ok(call_turn("accounts_list", "{}")),
            Ok(AiTurn {
                blocks: vec![Block::Text { text: "ok".into() }],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            }),
        ])
    };

    send(&session, &calls(), "first".into(), &mut |_| {}).unwrap();
    send(&session, &calls(), "second".into(), &mut |_| {}).unwrap();

    assert_eq!(gate.asked.borrow().as_slice(), ["accounts_list"], "asked once");
    assert_eq!(store.ai_grants_list(&chat_id).unwrap(), vec!["accounts_list"]);
}

#[test]
fn a_tool_that_does_not_exist_is_reported_to_the_model_not_to_the_user() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn("portfolio_teleport", "{}")),
        Ok(AiTurn {
            blocks: vec![Block::Text { text: "sorry".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    send(&session, &provider, "teleport".into(), &mut |_| {}).unwrap();

    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    let Block::ToolResult { content, .. } = &turns[2].blocks[0] else {
        panic!("expected a tool result");
    };
    assert!(content.contains("no tool called"));
}

#[test]
fn a_model_that_only_ever_calls_tools_stops_at_the_step_budget() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Session);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    // Always another call, never an answer.
    let provider = StaticAiProvider::turn(call_turn("app_periods", "{}"));

    send(&session, &provider, "loop forever".into(), &mut |_| {}).unwrap();

    assert_eq!(provider.seen.borrow().len(), MAX_STEPS);
    let last = super::super::store::turns(&store, &chat_id).unwrap();
    let Block::ToolResult { content, .. } = last.last().unwrap().blocks[0].clone() else {
        panic!("the budget is spent as a tool result, so the model can still answer");
    };
    assert!(content.contains("tool budget"));
}

#[test]
fn allowing_everything_switches_the_chat_and_later_tools_stop_asking() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Always);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let calls = |tool: &'static str| {
        StaticAiProvider::script(vec![
            Ok(call_turn(tool, "{}")),
            Ok(AiTurn {
                blocks: vec![Block::Text { text: "ok".into() }],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            }),
        ])
    };

    send(&session, &calls("accounts_list"), "first".into(), &mut |_| {}).unwrap();
    // A different tool: a per-tool grant would ask again, the chat's mode does not.
    send(&session, &calls("allocation_trees"), "second".into(), &mut |_| {}).unwrap();

    assert_eq!(gate.asked.borrow().as_slice(), ["accounts_list"]);
    assert_eq!(store.ai_chat_get(&chat_id).unwrap().tool_mode, AiToolMode::Auto);
    assert!(
        store.ai_grants_list(&chat_id).unwrap().is_empty(),
        "the mode is the permission; it is not also written per tool"
    );
}

#[test]
fn a_write_is_confirmed_every_time_whatever_the_chat_allows() {
    let (store, chat_id, scope) = fixture();
    // Everything a chat can be given: auto mode *and* a standing grant for the very tool.
    store.ai_chat_set_mode(&chat_id, AiToolMode::Auto).unwrap();
    store.ai_grant_add(&chat_id, "security_set_note").unwrap();

    let gate = ScriptedGate::new(Decision::Always);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let calls = || {
        StaticAiProvider::script(vec![
            Ok(call_turn(
                "security_set_note",
                r#"{"symbol":"IWDA.L","note":"x"}"#,
            )),
            Ok(AiTurn {
                blocks: vec![Block::Text { text: "ok".into() }],
                stop_reason: StopReason::EndTurn,
                usage: Usage::default(),
            }),
        ])
    };

    send(&session, &calls(), "first".into(), &mut |_| {}).unwrap();
    send(&session, &calls(), "second".into(), &mut |_| {}).unwrap();

    assert_eq!(
        gate.asked.borrow().as_slice(),
        ["security_set_note", "security_set_note"],
        "a write asks again every time, and \"allow all\" does not change that"
    );
}

#[test]
fn a_chat_in_auto_mode_never_asks_at_all() {
    let (store, chat_id, scope) = fixture();
    store.ai_chat_set_mode(&chat_id, AiToolMode::Auto).unwrap();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::script(vec![
        Ok(call_turn("positions_list", r#"{"limit":null}"#)),
        Ok(AiTurn {
            blocks: vec![Block::Text { text: "ok".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage::default(),
        }),
    ]);

    send(&session, &provider, "what do I hold?".into(), &mut |_| {}).unwrap();

    assert!(gate.asked.borrow().is_empty());
    let turns = super::super::store::turns(&store, &chat_id).unwrap();
    let Block::ToolResult { content, .. } = &turns[2].blocks[0] else {
        panic!("expected a tool result");
    };
    assert!(!content.contains("did not allow"), "it ran: {content}");
}

#[test]
fn the_first_message_names_the_chat_and_later_ones_leave_the_name_alone() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Deny);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);

    send(
        &session,
        &StaticAiProvider::text("ok"),
        "How did I do this year?".into(),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(
        store.ai_chat_get(&chat_id).unwrap().title,
        "How did I do this year?"
    );

    send(
        &session,
        &StaticAiProvider::text("ok"),
        "And last year?".into(),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(
        store.ai_chat_get(&chat_id).unwrap().title,
        "How did I do this year?"
    );
}

#[test]
fn a_long_first_message_is_cut_at_a_word_and_a_pasted_block_uses_its_first_line() {
    assert_eq!(title_from("  Short one  "), "Short one");
    assert_eq!(
        title_from("Which of my positions have lost the most money since I bought them?"),
        "Which of my positions have lost the most money…"
    );
    assert_eq!(title_from("\n\nFirst line\nsecond line"), "First line");
    // No whitespace to cut at: the title is still bounded.
    assert_eq!(title_from(&"x".repeat(200)).chars().count(), 49);
}

#[test]
fn a_provider_failure_is_the_call_s_error_and_does_not_lose_the_user_s_message() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let never = || false;
    let session = session(&store, &chat_id, &scope, &gate, &never);
    let provider = StaticAiProvider::failing(AiError::Auth("bad key".into()));

    let result = send(&session, &provider, "hi".into(), &mut |_| {});

    assert!(matches!(result, Err(AiError::Auth(_))));
    assert_eq!(super::super::store::turns(&store, &chat_id).unwrap().len(), 1);
}

#[test]
fn a_refused_or_cut_off_answer_is_kept_and_the_turn_says_why_it_ends() {
    for (stop, expected) in [
        (StopReason::Refusal, AiError::Refused),
        (StopReason::MaxTokens, AiError::Truncated),
    ] {
        let (store, chat_id, scope) = fixture();
        let gate = ScriptedGate::new(Decision::Once);
        let never = || false;
        let session = session(&store, &chat_id, &scope, &gate, &never);
        let provider = StaticAiProvider::turn(AiTurn {
            blocks: vec![Block::Text {
                text: "partial".into(),
            }],
            stop_reason: stop,
            usage: Usage::default(),
        });

        let result = send(&session, &provider, "hi".into(), &mut |_| {});

        assert_eq!(result.unwrap_err().to_string(), expected.to_string());
        // The question and what did arrive of the answer are both in the chat.
        assert_eq!(super::super::store::turns(&store, &chat_id).unwrap().len(), 2);
    }
}

#[test]
fn a_cancelled_turn_never_reaches_the_provider() {
    let (store, chat_id, scope) = fixture();
    let gate = ScriptedGate::new(Decision::Once);
    let always = || true;
    let session = session(&store, &chat_id, &scope, &gate, &always);
    let provider = StaticAiProvider::text("never sent");

    send(&session, &provider, "hi".into(), &mut |_| {}).unwrap();

    assert!(provider.seen.borrow().is_empty());
    // What the user typed is still theirs.
    assert_eq!(super::super::store::turns(&store, &chat_id).unwrap().len(), 1);
}
