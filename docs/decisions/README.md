# Architecture decision records

Every decision that shaped a module boundary, a stored format, an external integration or a
long-lived invariant is written down here, one file per decision, in the order it was taken. A
record says what the situation was, what was chosen, what else was considered and what the choice
costs. None of them is rewritten: when a decision changes, a new record supersedes the old one and
the old one keeps its reasoning and says who replaced it.

Read a record when you want to know *why* the code is shaped the way it is. For how to work in one
area of the code, read the matching file in [`.claude/rules/`](../../.claude/rules/); for what a
figure means to a user, read [`docs/user-guide/`](../user-guide/) and
[`docs/ai-reference/`](../ai-reference/).

New record: next free number, `NNNN-kebab-case-title.md`, the structure described in
[`CLAUDE.md`](../../CLAUDE.md#architecture-decision-records).

## Storage and the data model

| # | Decision | Status |
|---|---|---|
| [10](0010-rusqlite-over-orm.md) | `rusqlite` over an ORM | Accepted |
| [11](0011-sqlite-wal-and-busy-timeout.md) | WAL mode and busy_timeout on every connection | Accepted |
| [12](0012-plain-sql-migrations.md) | Migrations as a plain array of SQL files | Accepted |
| [13](0013-storage-design.md) | SQLite storage design | Accepted |
| [31](0031-instrument-attributes-are-typed-key-values.md) | Instrument attributes are typed key/values, not columns | Accepted |

## Market data and FX

| # | Decision | Status |
|---|---|---|
| [1](0001-ecb-as-fx-provider.md) | ECB as the FX provider | Accepted |
| [4](0004-fx-lookup-and-service-boundaries.md) | FX lookup and service boundaries | Accepted |
| [6](0006-market-provider-boundaries.md) | Keep market providers narrow and synchronous | Accepted |
| [7](0007-market-identifiers-and-listing-resolution.md) | Resolve listings through MIC and verified provider symbols | Accepted |
| [8](0008-market-quote-coverage.md) | Track quote coverage separately from returned quotes | Accepted |
| [9](0009-yahoo-market-adapter.md) | Normalize Yahoo wire data at the provider boundary | Accepted |
| [14](0014-single-background-refresh-job.md) | Market refresh is one background job broadcasting a global event | Accepted |
| [36](0036-the-quote-source-names-the-venue-it-quotes.md) | The quote source names the venue it quotes | Accepted |
| [50](0050-a-market-data-source-is-a-catalogue-row.md) | A market-data source is a catalogue row | Accepted |
| [51](0051-an-fx-rate-comes-from-the-first-source-that-publishes-the-pair.md) | An FX rate comes from the first source that publishes the pair | Accepted |
| [52](0052-a-fallback-quote-source-fills-gaps-and-never-rewrites-the-series.md) | A fallback quote source fills gaps and never rewrites the series | Accepted |
| [53](0053-market-sources-are-switched-and-keyed-by-the-user.md) | Market sources are switched and keyed by the user | Accepted |
| [54](0054-a-quote-source-the-user-describes.md) | A quote source the user describes | Accepted |
| [55](0055-a-keyed-source-s-daily-allowance-is-counted-in-the-profile.md) | A keyed source's daily allowance is counted in the profile | Accepted |

## Import

| # | Decision | Status |
|---|---|---|
| [2](0002-deterministic-import-preview.md) | Deterministic, user-overridable import preview | Accepted |
| [3](0003-taxonomy-csv-by-meaning.md) | Read taxonomy CSVs by meaning | Accepted |
| [5](0005-readable-import-fingerprints.md) | Human-readable import fingerprints | Accepted |
| [19](0019-csv-detection-by-language-and-values.md) | CSV import detection is per language and arbitrated by values | Accepted |
| [32](0032-a-taxonomy-is-seeded-from-an-attribute-not-derived-from-it.md) | A taxonomy is seeded from an attribute, never derived from it | Accepted |
| [61](0061-a-flex-statement-is-a-second-reader-not-a-second-import.md) | A Flex statement is a second reader, not a second import | Accepted |

## What the numbers mean

| # | Decision | Status |
|---|---|---|
| [18](0018-period-presets-belong-to-the-core.md) | Period presets belong to the core | Accepted |
| [20](0020-unlinked-cash-transfer-is-an-external-flow.md) | An unlinked cash transfer is an external flow | Accepted |
| [21](0021-contribution-divides-by-capital-at-work.md) | Contribution divides by the capital at work | Accepted |
| [22](0022-reports-are-a-window-not-a-lifetime.md) | Reports are a window, not a lifetime | Accepted |
| [24](0024-costs-paid-versus-charges-reported.md) | A cost rate counts the commission the cost basis swallowed | Accepted |
| [25](0025-absolute-performance-beside-contribution.md) | Absolute performance divides by the position's own capital | Accepted |
| [26](0026-user-defined-periods.md) | User-defined periods beside the shipped presets | Accepted |
| [27](0027-a-trade-is-built-from-consumed-lots.md) | A trade is built from the lots a disposal consumed | Accepted |
| [28](0028-currency-gain-beside-instrument-gain.md) | A result is split into the instrument's and the currency's share | Accepted |
| [43](0043-a-window-opens-on-what-was-there-before-it.md) | A reporting window opens on what was there before it | Accepted |
| [44](0044-a-lens-rewrites-the-leg-not-the-fact-of-the-trade.md) | A lens rewrites the leg, not the fact of the trade | Accepted |
| [56](0056-an-expected-dividend-is-last-year-carried-forward.md) | An expected dividend is last year's reported payment carried forward | Accepted |
| [57](0057-income-is-classified-by-its-payer.md) | Income is classified by its payer, not by what a category is worth | Accepted |
| [58](0058-both-cost-basis-methods-are-shown-side-by-side.md) | Both cost-basis methods are shown side by side, and neither becomes the portfolio's | Accepted |
| [60](0060-inflation-is-a-region-the-owner-names.md) | Inflation is a region the owner names, and a real return divides | Accepted |

## Portfolio features

| # | Decision | Status |
|---|---|---|
| [33](0033-an-investment-plan-proposes-transactions.md) | An investment plan proposes transactions, it does not own them | Accepted |
| [34](0034-an-alert-is-a-rule-whose-firing-is-derived.md) | An alert is a trigger level with a crossing log; events ride on the quote request | Accepted |
| [35](0035-a-watchlist-is-a-list-of-instruments-read-off-their-own-quotes.md) | A watchlist is a named list of instruments, read off their own quotes | Accepted |

## The host and the interface

| # | Decision | Status |
|---|---|---|
| [15](0015-network-calls-via-spawn-blocking.md) | Network lookups inside Tauri commands run through `spawn_blocking` | Accepted |
| [16](0016-store-passed-not-read-from-self.md) | `AppState::scoped_portfolio` takes `&Store` as an argument | Accepted |
| [17](0017-frontend-chart-rendering-architecture.md) | Frontend chart architecture | Accepted |
| [23](0023-ui-text-lives-in-the-frontend.md) | UI text lives in the frontend | Accepted |
| [29](0029-a-dashboard-widget-stores-one-size-in-twelfths.md) | A dashboard widget stores one size in twelfths | Accepted; resizing from the top and left edges superseded by [45](0045-a-dashboard-tile-is-resized-from-the-edge-being-dragged.md) |
| [30](0030-a-dashboard-widget-may-name-its-own-data-source.md) | A dashboard widget may name its own data source | Accepted |
| [45](0045-a-dashboard-tile-is-resized-from-the-edge-being-dragged.md) | A dashboard tile is resized from the edge being dragged | Accepted |
| [59](0059-a-tile-may-hold-assumptions-and-a-ratio-is-a-rendering.md) | A tile may hold assumptions, and a ratio of two shown figures is a rendering | Accepted |

## Profiles, keys and encryption

| # | Decision | Status |
|---|---|---|
| [46](0046-provider-keys-are-one-keychain-entry-read-once-per-run.md) | Provider keys are one keychain entry, read once per run | Superseded by [48](0048-provider-keys-live-in-a-password-sealed-profile-vault.md) on where keys are stored; the dev-signing runner still holds |
| [47](0047-a-profile-is-a-folder-of-its-own.md) | A profile is a folder of its own | Accepted |
| [48](0048-provider-keys-live-in-a-password-sealed-profile-vault.md) | Provider keys live in a password-sealed profile vault | Accepted; "the database stays unencrypted" superseded by [49](0049-a-protected-profile-keeps-its-database-encrypted.md) |
| [49](0049-a-protected-profile-keeps-its-database-encrypted.md) | A protected profile keeps its database encrypted | Accepted |

## The AI assistant

| # | Decision | Status |
|---|---|---|
| [37](0037-the-ai-assistant-lives-in-the-host.md) | The AI assistant lives in the host | Accepted; key storage superseded by [48](0048-provider-keys-live-in-a-password-sealed-profile-vault.md) |
| [38](0038-the-assistant-fetches-its-documentation.md) | The assistant fetches its documentation instead of carrying it | Accepted |
| [39](0039-the-dashboard-brief-is-a-reading-not-a-conversation.md) | The dashboard brief is a reading, not a conversation | Accepted; the ban on scheduled generation superseded by [40](0040-the-summary-tile-is-configured-like-a-widget.md) |
| [40](0040-the-summary-tile-is-configured-like-a-widget.md) | The summary tile is configured like a widget, schedule included | Accepted |
| [41](0041-what-a-turn-costs-is-counted-not-priced.md) | What a turn costs is counted, not priced | Accepted |
| [42](0042-a-provider-the-user-configures.md) | A provider the user configures | Accepted |
