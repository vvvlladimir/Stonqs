# 38: The assistant fetches its documentation instead of carrying it

- Status: Accepted

## Context

The assistant answers two kinds of question. "What is my portfolio worth" is data, and ADR-0037
settled how it is read: a catalogue of tools over `calc`, behind consent. "Why is my dividend
missing from this account" and "what does the period control change" are not data — they are how
this app works, and a model answering them from general knowledge invents something plausible.
An invention about someone's money, phrased with the confidence of documentation, is worse than
silence.

The obvious fix is a block of documentation in the system prompt. It does not survive contact with
the bill. A static prefix is paid for on the **first message of every chat**, whatever the question
turned out to be about; the provider's automatic prefix cache spares the later turns of one chat,
never the entry into a new one. A corpus covering every screen plus the domain rules is tens of
thousands of tokens, charged for a one-line question about a ticker. Worse, the cost grows with the
number of screens in the app rather than with the number of subjects the user actually raised.

The corpus is also small — dozens of short files, already organised by topic and by screen. A
retrieval layer with embeddings would be a search engine over a table of contents.

## Decision

Two corpora ship with the binary, and the model fetches one file at a time.

- `docs/ai-reference/*.md` — one file per domain concept, behind `app_reference(topic)`.
- `docs/user-guide/*.md` — one file per screen, behind `app_user_guide(screen)`.

Both are `Access::Free`: they read no portfolio data, so there is nothing to ask permission for,
and both are `app_*`, which is what the catalogue's test recognises a free tool by. Their contents
are `include_str!`-ed, so an edit ships with the next build and there is no directory to locate at
runtime on any of the five target platforms.

The **index** stays in the prompt while the **text** does not: the topic slugs and screen ids are
the `enum` of each tool's JSON Schema, so the model can only ask for something that exists, and the
list is static and caches with the rest of the prefix. The system prompt keeps only the rule that
makes the tools get used — documentation before explanation, and "I do not know how that part
works" when neither corpus answers.

Coverage of `docs/user-guide/` is deliberately partial. The freshness test therefore runs in the
direction that can be checked: **every guide file must name a screen that still exists**, not every
screen must have a guide. The first catches a screen renamed with its guide left behind; the second
would only block writing the corpus gradually. `SCREEN_IDS` is mirrored in the host and pinned
against the frontend's own union, the way `wire_format.rs` pins the wire.

What is true of *this request* — the date, the screen the user is on, the lens and base currency —
is `AiRequest::context`, a field of its own that the adapter renders **after** the static system
text. Everything before it is byte-identical between requests, which is the whole requirement the
prefix cache has.

## Alternatives

- **The whole corpus in the system prompt.** Simplest, and priced per chat rather than per subject.
  Rejected on cost, and on the growth curve: every new screen would tax every existing question.
- **A curated paragraph per concept inside the tool descriptions.** Free, since descriptions are
  already in the prefix — but descriptions are read by the model when *choosing* a tool, and
  padding them degrades that choice, which is the thing the catalogue exists to get right.
- **Embedding search over the corpus.** A vector store, an index to rebuild, a similarity threshold
  to tune, over a few dozen files whose names already say what is in them.
- **Pointing the tools at `.claude/rules/` and `docs/decisions/` directly.** Zero new text, and it
  was the original plan. Rejected: those are written for whoever edits this repository and name
  structs, modules, migrations and files. The model repeats what it reads, and a user asking about
  their dividends should not be told about `Holdings::fees_base`. The corpora here are the same
  knowledge with the internals removed.

## Consequences

- Two new directories of prose that are neither UI strings nor developer notes. They are English,
  like every other string in `core`, `app/src-tauri` and `cli`, and are never rendered — the model
  answers in the user's language from them, so no catalogue is involved (ADR-0023 is untouched).
- A chat's cost now grows with the number of subjects raised in it, not with the size of the app.
- A guide can go stale against the screen it describes, and nothing mechanical catches that — the
  same footing `.claude/rules/` and the ADRs are already on: it holds through review and proximity
  in the diff.
- The tool results are fenced in `<tool_data>` like every other result, including these, which the
  repository itself wrote. The fence's promise is that the model takes no instruction from a tool
  result, and making an exception for one source is how that promise stops being checkable.
