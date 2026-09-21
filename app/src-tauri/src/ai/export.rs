//! A chat as one Markdown file. The panel renders markdown already, so exporting it is writing
//! down what is on screen — plus the tool calls and their readings, which is the part a chat kept
//! outside the app is worth keeping for.
//!
//! Raw HTML is deliberately absent, the same choice `components/ui/Markdown.tsx` makes: this text
//! was written by a model that had read somebody's CSV, and a file that renders script tags in a
//! notes app is the same hazard as a panel that does.
//!
//! The few English words here (`You`, `Assistant`) are of a piece with the CSV exports' column
//! names — a saved file is not IPC, and nothing in the app reads this back.

use super::store::ChatTurn;
use super::{Block, Role};
use sq_core::model::AiChat;
use std::fmt::Write;

pub fn markdown(chat: &AiChat, turns: &[ChatTurn]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {}\n", chat.title);
    let _ = writeln!(
        out,
        "{} · {} · {} — {}\n",
        chat.provider, chat.model, chat.created_at, chat.updated_at
    );

    for turn in turns {
        // A turn carrying nothing but tool results is the loop handing readings back, not a side
        // of the conversation — the panel does not draw it either. Its results are written under
        // the call that asked for them.
        if turn.blocks.iter().all(|b| matches!(b, Block::ToolResult { .. })) {
            continue;
        }
        let who = match turn.role {
            Role::User => "You",
            Role::Model => "Assistant",
        };
        let _ = writeln!(out, "## {who} · {}\n", turn.created_at);
        for block in &turn.blocks {
            write_block(&mut out, block, turns);
        }
    }
    out
}

fn write_block(out: &mut String, block: &Block, turns: &[ChatTurn]) {
    match block {
        Block::Text { text } => {
            let _ = writeln!(out, "{}\n", text.trim_end());
        }
        Block::Reasoning { text, .. } => {
            let _ = writeln!(out, "> **Thinking**");
            for line in text.trim_end().lines() {
                let _ = writeln!(out, "> {line}");
            }
            out.push('\n');
        }
        Block::WebSearch { query } => {
            let _ = writeln!(out, "**Web search:** {query}\n");
        }
        Block::ToolCall {
            id, name, args_json, ..
        } => {
            let _ = writeln!(out, "**{name}**\n");
            fenced(out, args_json);
            // The result arrives in a later turn, so it is found by call id across the whole
            // chat rather than beside the call — the same join the panel makes.
            if let Some(content) = result_of(turns, id) {
                fenced(out, content);
            }
        }
        // Reached only for a result whose turn also held something else; the join above already
        // wrote every other one.
        Block::ToolResult { content, .. } => fenced(out, content),
    }
}

/// A fence long enough to survive whatever the model or a tool put inside — a reading quoting a
/// note that itself contains ``` would otherwise end the block early. The longest run anywhere
/// counts, not only one at the start of a line: a run is cheap to measure and a fence one
/// backtick too short is a file that renders as nonsense.
fn fenced(out: &mut String, body: &str) {
    let mut longest = 0;
    let mut run = 0;
    for ch in body.chars() {
        run = if ch == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest.max(2) + 1);
    let _ = writeln!(out, "{fence}\n{}\n{fence}\n", body.trim_end());
}

fn result_of<'a>(turns: &'a [ChatTurn], call_id: &str) -> Option<&'a str> {
    turns
        .iter()
        .flat_map(|t| &t.blocks)
        .find_map(|block| match block {
            Block::ToolResult { call_id: id, content } if id == call_id => Some(content.as_str()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat() -> AiChat {
        AiChat {
            id: "chat-1".into(),
            title: "How did I do this year?".into(),
            provider: "openai".into(),
            model: "gpt-5.1".into(),
            tool_mode: sq_core::model::AiToolMode::Ask,
            effort: sq_core::model::AiEffort::Medium,
            created_at: "2026-09-15 10:00:00".into(),
            updated_at: "2026-09-15 10:02:00".into(),
        }
    }

    fn turn(role: Role, blocks: Vec<Block>) -> ChatTurn {
        ChatTurn {
            id: "m".into(),
            chat_id: "chat-1".into(),
            role,
            blocks,
            created_at: "2026-09-15 10:00:00".into(),
        }
    }

    #[test]
    fn a_reading_is_written_under_the_call_that_asked_for_it() {
        let turns = vec![
            turn(
                Role::User,
                vec![Block::Text {
                    text: "How did I do?".into(),
                }],
            ),
            turn(
                Role::Model,
                vec![Block::ToolCall {
                    id: "call_1".into(),
                    name: "portfolio_overview".into(),
                    args_json: "{\"period\":\"YTD\"}".into(),
                    signature: None,
                }],
            ),
            turn(
                Role::User,
                vec![Block::ToolResult {
                    call_id: "call_1".into(),
                    content: "{\"twr_percent\":\"7.10\"}".into(),
                }],
            ),
            turn(
                Role::Model,
                vec![Block::Text {
                    text: "Up 7.1% this year.".into(),
                }],
            ),
        ];
        let out = markdown(&chat(), &turns);

        let call = out.find("portfolio_overview").expect("the call");
        let reading = out.find("twr_percent").expect("the reading");
        let answer = out.find("Up 7.1%").expect("the answer");
        assert!(call < reading && reading < answer, "{out}");
        // The turn that only carried the result is not a side of the conversation.
        assert_eq!(out.matches("## You").count(), 1, "{out}");
    }

    #[test]
    fn a_reading_containing_a_fence_does_not_end_the_block_early() {
        let turns = vec![turn(
            Role::Model,
            vec![Block::ToolCall {
                id: "call_1".into(),
                name: "securities_note".into(),
                args_json: "{}".into(),
                signature: None,
            }],
        )];
        let mut turns = turns;
        turns.push(turn(
            Role::User,
            vec![Block::ToolResult {
                call_id: "call_1".into(),
                content: "note: ```rm -rf```".into(),
            }],
        ));
        let out = markdown(&chat(), &turns);
        assert!(out.contains("````\nnote: ```rm -rf```\n````"), "{out}");
    }

    #[test]
    fn the_models_thinking_is_quoted_line_by_line() {
        let turns = vec![turn(
            Role::Model,
            vec![
                Block::Reasoning {
                    text: "First this.\nThen that.".into(),
                    signature: None,
                },
                Block::Text { text: "Done.".into() },
            ],
        )];
        let out = markdown(&chat(), &turns);
        assert!(out.contains("> First this.\n> Then that.\n"), "{out}");
        assert!(out.contains("\nDone.\n"), "{out}");
    }
}
