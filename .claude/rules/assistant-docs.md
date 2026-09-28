---
paths:
  - "docs/user-guide/**"
  - "docs/ai-reference/**"
  - "app/src-tauri/src/ai/guide.rs"
  - "app/src/screens/**"
  - "app/src/lib/nav.tsx"
---

# Assistant docs (`docs/user-guide/`, `docs/ai-reference/`)

The AI assistant's only source of truth about the app, fetched via `app_user_guide(screen)` / `app_reference(topic)` (ADR-0038). A stale file makes it confidently wrong.

- `docs/user-guide/<ScreenId>.md` — one screen (id from `app/src/lib/nav.tsx`): what it shows, what controls change, traps.
- `docs/ai-reference/<topic>.md` — one concept: what a figure means, why two similar figures differ, what makes one absent.
- A fact for one screen → its guide; a fact several screens rely on → reference, and guides name it.

**Update in the same change** when: a screen's behaviour changes (control meaning, figure window, writes vs proposes); a screen is added/renamed/removed (file follows; `SCREEN_IDS` is pinned to `nav.tsx`); a new user-visible invariant/ADR; a new `ai/tools/` concept the corpus doesn't explain. **Not** for refactors, restyles, a column of an already-described kind, or bug fixes restoring documented behaviour.

Writing:
- Verify against the screen/calculation first; no intent or half-finished features.
- No internals (modules, structs, tables, commands, paths, code).
- Lead with traps: scoped vs not, "as of today" vs "over a period", absent ≠ zero, writes vs proposes.
- Name controls by their exact English label; describe what they do, not where they sit (no dashboard positions).
- English, present tense, short paragraphs, ~250–500 words, one subject. No screenshots, click-tours or shortcut lists.

Register every file in `GUIDES` or `REFERENCE` in `app/src-tauri/src/ai/guide.rs` (slug = file name; `REFERENCE` slugs are the `app_reference` enum). Tests: `every_file_in_both_corpora_is_reachable`, `every_guide_is_named_after_a_screen_that_exists`. A screen without a guide is allowed (assistant says it doesn't know).
