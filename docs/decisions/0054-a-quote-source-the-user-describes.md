# 54: A quote source the user describes

- Status: Accepted

## Context

No catalogue ships every source a user may have: a regional exchange's JSON, a bank's fund page,
a paid API this build has no adapter for. Other trackers answer this with a "JSON" feed
(URL plus JSONPath) or custom providers (URL template, JSON/HTML/CSV extraction,
secret headers). Both are data, not code.

## Decision

- `market::CustomSource` describes a quote source as data: an `https` URL template
  (`{SYMBOL} {ISIN} {MIC} {CURRENCY} {FROM} {TO} {FROM:%s} {TO:%s} {KEY}`), headers, a format —
  `Json { date_path, close_path }` (two paths zipped by position) or `Csv { date_column,
  close_column }` — an optional date format, factor and currency. `CustomProvider` implements
  `QuoteProvider` over it, so the chain, the guard and the breaker treat it like any other source.
- The JSONPath is the subset a price feed needs (`$ .name ['name'] [n] [*] .*`), implemented in
  the module rather than pulled in as a dependency. CSV goes through the import's own parser, so
  delimiter, encoding and decimal detection are the import's.
- Ids are `custom:<slug>`; a custom source can never shadow a shipped one. Definitions live in
  `AppSettings::market_custom` (per profile); a key lives in the vault under `market:<id>` and only
  `{KEY}` in the template or a header names it.
- Only `https`, no redirects, 20 s timeout, answers over 5 MB are cut off. A closes path that
  reads nothing usable is `BadProviderData`, not an empty week.
- `market_custom_test` asks an unsaved definition for 30 days of one symbol and returns ten rows,
  so the form shows what the paths read before anything is stored.

## Alternatives

- **Scripts (JS/WASM/Lua).** Unlimited reach, and unlimited ways to hurt the user; the declarative
  form covers the feeds people actually paste.
- **HTML scraping by CSS selector.** Breaks with every redesign of the page; left out for now.
- **A JSONPath crate.** A dependency for five path steps.

## Consequences

- A user can add any JSON or CSV price endpoint without a build. Custom sources are quote sources
  only; an FX variant (`{BASE}`, `{QUOTE}`) would be the same shape and is not built yet.
