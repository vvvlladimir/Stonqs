# 23: UI text lives in the frontend

- Status: Accepted

## Context

The application shipped with Russian written into every layer. The frontend carried about a
thousand literals, and the Rust side produced display text of its own: `UiError` messages,
`scope.rs` composing `"Депо · A + B"`, `jobs.rs` labelling a refresh failure,
`calc/allocation.rs` naming its generated buckets `"не классифицировано"` and
`"денежные средства"`, `import/checks.rs` phrasing every warning.

Adding a second language exposes the real problem: a string produced in `core` or in the Tauri
host cannot follow a preference that lives in the UI. The catalog would have to exist twice, in
two toolchains, and `core` would have to learn which language a caller speaks — the opposite of
"the core never prints".

## Decision

**Rust emits data; the frontend writes sentences.** Concretely:

- `ScopeOption` carries `kind`, `name`, `account_kind` and `cash_name`. `ScopePicker.scopeLabel`
  is the single place that composes the wording around them.
- `calc::allocation` labels its two generated buckets with their own key (`UNCLASSIFIED`,
  `CASH`); `lib/taxonomy.bucketLabel` turns a key into a word. A bucket that comes from a
  taxonomy node keeps the node's name, which is user data.
- A refresh failure is `Failure { code, subject, detail }`. `MarketRefresh.headline` writes the
  headline from `code`; `subject` is a symbol or a currency pair, `detail` the underlying error.
- `ImportProblem` keeps its `code` and gains `params` — the values behind the sentence.
  `Import/labels.problemDetail` writes the sentence; `message` stays as the English fallback for
  a code the UI has no sentence for.
- `UiError` is unchanged: it already carried a `code`. `components/ui/Async.useErrorText` writes
  the headline from the code and appends the host's English `message` as detail.
- Every remaining string in `core`, `app/src-tauri` and `cli` is English. They are developer
  text: an error detail, a log line, a CLI harness.

English is the source language of the frontend catalog, so an untranslated message degrades to
English rather than to a key.

## Alternatives

- **A translation runtime in Rust (`fluent`, `rust-i18n`).** Two catalogs, two extraction
  toolchains, and `core` would need the active locale threaded through every call — it would
  have to know about a UI preference to do arithmetic.
- **Passing the locale to each command.** Same coupling, plus every command gains an argument
  that has nothing to do with what it computes.
- **Leaving the text in Rust and translating it there once.** Works for exactly two languages
  and breaks on the third, which is the case this decision exists for.

## Consequences

- A new user-visible sentence is a frontend change, even when the fact behind it is computed in
  the core. The core's job is to name the fact (a code, a key, the values).
- The wire format is wider in places: a code plus parameters is more fields than one string.
  That is the price of the sentence being written where the language is known.
- CSV exports (`report_export`) carry English column headers. A spreadsheet column is not UI
  text that can follow a preference: the file outlives the session that produced it.
- Test assertions match codes, not wording, so rephrasing a message cannot break a test.
