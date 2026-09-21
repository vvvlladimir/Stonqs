<!-- Keep whichever lines apply and delete the rest. A one-line fix does not need a long form. -->

## What this changes

<!-- What the user can now do, or what stopped being wrong. Link the issue with "Closes #NN". -->

## Why this way

<!-- Only if the approach is not the obvious one, or if you considered something else and dropped
     it. Skip this whole section otherwise. -->

## Checks

- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
- [ ] In `app/`: `pnpm lint`, `pnpm lint:css`, `pnpm format:check`, `pnpm build`, and
      `pnpm i18n:extract` reports no missing translations.
- [ ] I read the file in [`.claude/rules/`](../.claude/rules/) for the area I touched.
- [ ] New user-visible text is in the frontend, in English, behind a Lingui macro — not in Rust.

## Obligations this change may carry

- [ ] **Tests.** A new calculation has a test that starts with the arithmetic worked out longhand in
      a comment. A fixed defect has a test that fails without the fix.
- [ ] **ADR.** This changes a module boundary, a stored format, an external integration or a
      long-lived invariant, so it adds one in [`docs/decisions/`](../docs/decisions/) — or it does
      none of those, and needs none.
- [ ] **Documentation.** This changes what a screen does or what a figure means, so the matching
      file under `docs/user-guide/` or `docs/ai-reference/` is updated in the same change — or it
      changes neither. The in-app assistant answers from those files.
- [ ] **Migration.** This adds `000N_*.sql` and a row in `MIGRATIONS` rather than editing an applied
      migration, and `.claude/rules/migrations.md` describes it.

## Anything a reviewer should know

<!-- A screenshot for a visual change. A figure you are unsure about. A follow-up you deliberately
     left out. "Nothing" is a fine answer. -->
