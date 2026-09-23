# Contributing to Stonqs

Thanks for looking. This is an alpha-stage desktop application that people point at their real
money, so the bar for a change is "I can show why this number is right", not "it compiles".

Before a large change, open an issue or a discussion first. A refused pull request wastes more of
your evening than a refused idea.

Taking part here means following the [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting set up

```bash
git clone git@github.com:vvvlladimir/stonqs.git
cd stonqs
git config core.hooksPath .githooks       # commit message check
git config commit.template .gitmessage    # the template it checks against

cargo build --workspace                   # Rust: core, cli, the Tauri host
cd app && pnpm install                    # frontend
pnpm tauri dev                            # the app, with a dev server on :1420
```

You need a recent stable Rust (edition 2024), Node with pnpm, and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. On Linux that
is the WebKitGTK and AppIndicator development packages; on Windows the MSVC build tools.

## Before you open a pull request

Everything below must pass. It is what CI runs, and it runs offline — tests that need the network
are `#[ignore]`d.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd app
pnpm lint && pnpm lint:css && pnpm format:check
pnpm build
pnpm i18n:extract     # must report no missing translations
```

## How this repository is organised

Read [`CLAUDE.md`](CLAUDE.md) first — it is the map: which crate is allowed to know about which,
and where a given kind of code belongs. The per-area rules live in [`.claude/rules/`](.claude/rules/)
(money and FX, import, migrations, the UI boundary, the frontend, the assistant). Read the file for
the area you are touching before you touch it; most review comments on a first pull request are
things written down there.

The one-way rule matters more than anything else here: `core` is a library that knows nothing about
Tauri, the network layer knows nothing about the calculations, and the calculations know nothing
about SQLite. `grep -r tauri core/` must stay empty.

A few rules with teeth:

- **Every calculation test starts with the arithmetic worked out longhand in a comment**, then the
  same computation in code. See `core/tests/calc_examples/`. A failing test should be readable
  without a debugger.
- **User-visible text lives in the frontend**, in English, behind a Lingui macro. Rust sends codes
  and values, never sentences.
- **Money is `Decimal`, never `f64`**, and a price never travels without its currency.
- **An applied migration is never edited** — add a new one.
- **A decision that changes a module boundary, a stored format or a long-lived invariant needs an
  ADR** in [`docs/decisions/`](docs/decisions/). Start at that folder's README; a routine fix or a
  local refactor does not need one.
- **Changing what a screen does, or what a figure means, obliges the matching file** under
  `docs/user-guide/` or `docs/ai-reference/` in the same change. The in-app assistant answers from
  those files, so a stale one makes it confidently wrong rather than merely vague.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/), so the changelog is generated from
the history instead of written by hand:

```
fix(storage): back up the database before an upgrade

A sequence of migrations cannot be undone, and a lost portfolio is nobody
else's to restore. The copy is taken after a WAL checkpoint.

Refs: ADR-0062
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`. Scopes are the
areas of the repository: `core`, `app`, `ui`, `calc`, `storage`, `market`, `fx`, `import`, `ai`,
`dashboard`, `i18n`, `docs`, `deps`. The subject is imperative, at most 72 characters, no full
stop. The `commit-msg` hook above checks all of that locally.

Say *why* in the body. What changed is visible in the diff; why it had to is not.

Those subjects are what the changelog is made of, so the ones users read are `feat` and `fix`;
everything else is kept out of it. A `!` after the scope and a `BREAKING CHANGE:` paragraph are
what move the major version.

## Releases

Nobody tags by hand. A bot keeps one open pull request holding the next version number and the
changelog entries earned since the last release; merging it writes `CHANGELOG.md`, bumps the
version everywhere it is written down, tags, and opens a draft release. The installers are built
from that tag and attached to the same draft, which a maintainer publishes after looking at it.

Publishing is also what makes the update feed visible, so a build nobody has checked cannot be
offered to anybody. See [ADR-0063](docs/decisions/0063-an-update-is-offered-never-applied.md).

## Adding a broker

Broker layouts are data, not code: `core/presets/brokers.json`. Detection is per *language*, never
per broker — a rule keyed to one broker's export helps only that broker's customers, so header
aliases and operation wordings go into the shared dictionaries.

If your broker's export does not import cleanly, open a **broker format** issue rather than a pull
request, and **strip it first**: remove real amounts, account numbers and names. Five rows and the
header line are enough to work from, and you should assume anything you attach is public forever.

## AI-assisted contributions

They are welcome, and this repository was itself built with heavy use of them. The condition is
simple: you have read every line you are submitting and you can defend it in review. A pull request
whose author cannot explain why a calculation is correct will be closed regardless of who or what
wrote it — the same standard as for hand-written code, applied honestly.

By contributing you agree to the [Contributor License Agreement](.github/CLA.md); a bot will ask
you to sign it once, on your first pull request.

## Security

Do not open a public issue for a vulnerability. Write to security@stonqs.app or see
[SECURITY.md](SECURITY.md).
