# AI reference corpus

Short, self-contained explanations of the domain concepts Stonqs reasons in, written **for the
assistant**, not for a developer. One file per topic; the file name is the `topic` id the
`app_reference` tool takes.

Two rules keep this corpus useful:

- **No internals.** No module names, no struct names, no SQL, no file paths. The assistant answers
  a person who has never seen this repository; anything it reads here it may repeat out loud.
- **Explain the meaning, not the steps.** How a screen is operated belongs in `docs/user-guide/`.
  What a number *is* and why it behaves the way it does belongs here.

A topic that is only ever needed on one screen belongs in that screen's guide instead. A topic the
assistant would otherwise guess at — and guess plausibly but wrongly — belongs here.

When to add a topic, how to write one and how to register it: `.claude/rules/assistant-docs.md`.
