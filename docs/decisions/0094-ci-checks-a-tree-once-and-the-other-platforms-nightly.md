# 94: CI checks a tree once, and the other platforms nightly

- Status: Accepted

## Context

Every push to main reran all of CI on Linux, macOS and Windows, even when the squash merge landed
exactly the tree its pull request had just passed, and every pull request ran all three operating
systems as well. Each job wrote its own cache for every new `Cargo.lock`, pull requests included,
so the caches crowded one another out of GitHub's 10 GB. A release restored nothing: its caches
from the release before had long been evicted. The fuzz job compiled `cargo-fuzz` from source on
every run.

## Decision

- Linux (`rust (ubuntu)`, `import`, `frontend`) runs on a pull request and on a push to main, but a
  push is skipped when an earlier run's three Linux jobs passed on the same git tree. A run passing
  is not enough: a draft's run passes with every job skipped.
- macOS and Windows run nightly, only when main moved since the last nightly run that passed, and
  on any branch through `workflow_dispatch`. The nightly run also runs Linux, which keeps main's
  cache fresh when merges were skipped.
- A change only to files nothing builds — root Markdown, `docs/` except `docs/ai-reference/` (the
  assistant's guide compiles it in), `.claude/`, the issue templates — runs none of
  it, the nightly run included.
- Only main saves a cache; the release workflow keeps none. CI builds without debug info.
- A tool is downloaded prebuilt by `taiki-e/install-action`, never compiled by `cargo install`.

## Alternatives

- Raise the cache limit: storage above 10 GB is billed, and it leaves the duplicate runs in place,
  which cost more than the evictions.
- `fkirc/skip-duplicate-actions`: dedupes by tree as well, but counts a run as passed by its
  conclusion, so a draft's empty run would let the ready pull request skip Linux.

## Consequences

A merge of an up-to-date pull request costs a gate job instead of five jobs. A macOS- or
Windows-only break is found by the next morning rather than right after the merge; when the day's
merges touched platform code (keychain, signing, paths), dispatch CI on the release pull request
before merging it. Reopen this if a nightly failure lands in a release.

A pull request's second push restores main's cache rather than its own first push's, so it
compiles somewhat more. Tool versions are whatever is latest, as with `cargo install` before.
