# 50: A market-data source is a catalogue row

- Status: Accepted

## Context

The host assembled its sources by hand in `jobs.rs` — Yahoo for quotes and search, OpenFIGI for
listings, the ECB for rates — and the ids were string literals scattered across commands, the AI
tools, the import defaults and the refresh job (`"yahoo"`, `"ecb"`). Stooq existed in the core but
was reachable only from the CLI.

The app is about to grow from one source per role to many — keyless and keyed quote APIs, crypto
exchanges, further FX publishers, eventually a source the user describes — and to try them as a
chain rather than as one fixed answer. Neither is possible while "which sources exist" is spelled
in five files and "which one answers" is a literal at each call site.

Errors were also too coarse for a chain: a 429 and a 503 were both `Unavailable`, a rejected key
was the same `Network` as a 404. A chain must rest a throttled source and stop asking one whose
key is wrong, and it cannot tell those apart from the error it gets.

## Decision

`sq_core::sources` is the one place that knows which sources ship. Each `SourceInfo` row carries
the stored id, what an API key means to it (`KeyUse::None | Optional | Required`), whether it is on
by default, and one constructor per role it can play (quotes, search, listings, FX). Its
capabilities are derived from those constructors rather than declared beside them.
`sources::quote_service()` and `sources::fx_service()` build the services from the rows that are
on; `DEFAULT_QUOTES` and `DEFAULT_FX` name the source a new instrument or a currency pair uses.

A provider's id is its `ID` constant, used by its `id()`, by the rows it writes, and by the
catalogue — so the id stored in the database and the id in the catalogue cannot drift.

`Error` gains `RateLimited` (429, still transient, so today's retries are unchanged) and
`Unauthorized` (401/403, never retried). The UI maps them to the causes it already had.

## Alternatives

- **Keep building services in the host.** The core would stay unaware of what exists, and every
  new source would touch the host, the CLI and the import defaults separately.
- **A trait object per source with `capabilities()` declared by hand.** Two statements of the same
  fact, one of which is wrong the first time someone adds a role and forgets the list.
- **Configuration file of sources.** A user-described source will need data, but the shipped ones
  are code: each is an adapter to one wire format, and a file cannot supply that.

## Consequences

- Adding a shipped source is a provider file plus a row; nothing else in the workspace names it.
- A source kept off (`on_by_default: false`, Stooq today) still has a row, so an instrument that
  names it is a known, disabled source rather than an unknown string.
- Behaviour is unchanged by this decision: the same three sources are on and each role is answered
  by one of them. Trying the next source on failure, per-source symbols, rate budgets and keyed
  sources build on this seam and are decided separately.
- The frontend still defaults a new instrument to `"yahoo"` itself (`Securities/model.ts`,
  `Import/index.tsx`); that moves to the host once the preferred source becomes optional.
