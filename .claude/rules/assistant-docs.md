# The assistant's documentation (`docs/user-guide/`, `docs/ai-reference/`)

Two corpora the AI assistant fetches at runtime through `app_user_guide(screen)` and
`app_reference(topic)` — see ADR-0038. They are **the assistant's only source of truth about how
this app works**: the system prompt tells it to read them before explaining anything and to say
"I do not know" when they answer nothing. A screen whose behaviour changed without its guide
changing does not produce a vague answer, it produces a confident wrong one.

Neither corpus is `.claude/rules` and neither is a changelog. `.claude/rules` and ADRs are written
for whoever edits this repository; these two are written for a model that will paraphrase them to
somebody who has never seen the code.

## Which corpus

- `docs/user-guide/<screen_id>.md` — **one screen**: what it shows, what its controls change, what
  trips people up. File name is the frontend's `ScreenId` (`app/src/lib/nav.tsx`), nothing else.
- `docs/ai-reference/<topic>.md` — **one concept** the app reasons in, independent of where it is
  displayed: what a figure means, why two figures that look alike differ, what makes one absent.
  File name is the `topic` id.

A fact needed on exactly one screen goes in that screen's guide. A fact three screens rely on goes
in the reference, and the guides name the concept instead of re-explaining it — the model can fetch
the other file itself, and two copies of an explanation drift apart.

## When to write or update one — in the same change, not later

- **A screen's behaviour changes** — a control starts meaning something else, a figure is computed
  over a different window, an action starts or stops writing data. Update that screen's guide in
  the same commit.
- **A new screen** — add its guide with it. `SCREEN_IDS` in `ai/guide.rs` is pinned to `nav.tsx`, so
  the id is already decided for you.
- **A screen is renamed or removed** — rename or delete the file. A guide named after a screen that
  no longer exists fails `every_guide_is_named_after_a_screen_that_exists`.
- **A new invariant about what a number means** (a new ADR in the domain, a new `.claude/rules`
  entry that a user could feel) — that is a reference topic, in the user's vocabulary, not the
  repository's.
- **A new tool in `ai/tools/` exposing a concept the corpus never explains** — add the topic, or
  the model will explain its own output from general knowledge.

Do **not** touch either corpus for a refactor, a restyle, a renamed component, a new column that is
one more of a kind already described, or a bug fix that restores documented behaviour. The test is
whether the answer to a user's question changed, not whether the markup did.

## How to write one

- **Verify against the screen or the calculation before writing.** Everything here is repeated to a
  user as fact. Do not document intent, a plan, or a half-finished feature.
- **No internals.** No module, struct, table, command or file names; no Rust, no SQL, no paths. If a
  sentence only makes sense to someone with the repo open, it is in the wrong corpus.
- **Lead with the traps.** The value of these files is the part a model would otherwise get
  plausibly wrong: what is scoped and what is not, which figure is "as of today" beside one that is
  "over a period", where an absent value is not a zero, which action writes data and which only
  proposes it.
- **Name a control by the app's own English label, exactly** (`New transaction`, `Identify`,
  `Refresh quotes`). The interface is translated and these files are not, so a paraphrase can never
  be matched to the translated label the user is actually looking at. Prefer saying what a control
  *does* over saying where it sits — position changes, meaning does not; the dashboard has no fixed
  layout at all, so never describe a tile's place on it.
- **Short paragraphs, present tense, English, roughly 250–500 words.** The text is paid for on every
  fetch. One subject per file; do not summarise a neighbouring file inside this one.
- No screenshots, no click-by-click tour, no keyboard shortcuts list — all three go stale at the
  first UI change and none of them answers "why is this number what it is".

## Registering a file

A file that is not registered is invisible to the assistant, and
`every_file_in_both_corpora_is_reachable` fails. Add the tuple to `GUIDES` or `REFERENCE` in
`app/src-tauri/src/ai/guide.rs` (slug = file name) in the same change. `REFERENCE`'s slugs are the
`app_reference` schema's `enum`, so a new topic is offered to the model by adding it there and
nowhere else.

Both are `include_str!`-ed: an edit ships with the next build, and there is no directory to find at
runtime on any platform.

## What the corpus does not promise

Coverage is complete today and the test still runs the other way round — every file must name a
screen that exists, never "every screen must have a file". A screen shipped before its guide is
written is a known state, not a bug: the tool answers nothing and the prompt turns that into "I do
not know how that part works", which is the correct answer and the whole reason absence is allowed.
