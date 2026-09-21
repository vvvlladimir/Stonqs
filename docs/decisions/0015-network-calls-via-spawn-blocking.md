# 15: Network lookups inside Tauri commands run through `spawn_blocking`


- Status: Accepted

## Context

`sq-core` is deliberately synchronous — no async, no tokio (see
`CLAUDE.md`: "Providers stay one-method... Everything is synchronous").
Some Tauri commands still need the network directly: security search/resolve,
ISIN identification, listing discovery. A synchronous Tauri command runs on
the main thread, so calling a blocking HTTP client from it freezes the UI for
the whole request.

## Decision

Those commands are declared `async fn`, and the network-calling body is
dispatched with `tauri::async_runtime::spawn_blocking`, which runs it on a
blocking-task pool while the command `.await`s the result. The core stays
synchronous everywhere; only the host wraps the call.

Any lock the command needs is taken and dropped **before** the
`spawn_blocking` call — the same rule as the background refresh job (see
`0080-single-background-refresh-job.md`), applied per-command instead of as
one global job.

Applies to: `commands/listings.rs::security_listings`,
`commands/lookup.rs::security_search`/`security_resolve`/
`import_resolve_symbol`, `commands/securities.rs::security_identify`.
