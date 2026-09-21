# User guide

One file per screen, named after the screen's id. The assistant reads these through the
`app_user_guide` tool when a question is about how the app works rather than about the user's
numbers.

Write about the screen as a person meets it: what it shows, what its controls change, and what
trips people up. Not a click-by-click tour — those go stale at the first UI change — and not the
domain concepts behind the figures, which live in `docs/ai-reference/`.

Every screen in the app has a file here, and a test checks the other way round — a file named
after a screen that no longer exists fails the build. A screen added without a guide is not a bug
either: the tool answers "no guide for this screen", and the assistant says it does not know rather
than inventing something plausible.

When a screen changes, its file here changes in the same commit — the rule for that, and for
writing and registering a new one, is `.claude/rules/assistant-docs.md`.
