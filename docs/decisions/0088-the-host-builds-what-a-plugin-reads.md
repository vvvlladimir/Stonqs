# 88: The host builds what a plugin reads, for a page and a tool alike

- Status: Accepted

## Context

ADR-0083 had the frontend feed a plugin's frame: the same query hooks as the built-in tiles, their
results projected into the bridge's field names by `lib/pluginBridge.ts`, so a plugin tile beside a
positions list cost no second query. ADR-0085 then needed the same data inside an assistant turn,
which runs in the host with no window, and built it a second time in Rust — two copies of one
shape, held together by a test that compared their field names and nothing else. The copies had
already begun to differ in behaviour: the frontend's `xirr` failed on a missing price, the host's
quietly became `null`.

## Decision

`plugins::reads::project(store, scope, reads, date, period)` is the one place the reads are built.
A page's frame gets them through the `plugin_reads` command — one query per frame
(`usePluginReads`), keyed with the reads, date, period and data source and invalidated with the
reports — and an assistant tool through `ai::tools::plugin`, from the same function. The TypeScript
interfaces in `lib/pluginBridge.ts` stay as the types of what arrives, and
`the_projection_is_the_bridges` still pins them to what the host builds. Data remains pushed, never
asked for: the frame still sends no request, the host builds only what the manifest declared.

## Alternatives

- **Keep two projections and compare values, not just names.** A test that runs the frontend's
  projection needs a JavaScript runtime inside `cargo test`, and every new read would still be
  written twice.
- **Build tools' data in the frontend.** An assistant turn would need a round trip through the
  window for every call and would die with a reload (ADR-0085).

## Consequences

- A plugin tile no longer reuses a built-in tile's query: beside a positions list the positions
  are computed twice. For a portfolio of ordinary size the cost is one more holdings pass per tile.
- ADR-0083's "through the same query hooks and keys as the built-in tiles" and ADR-0085's "two
  copies of one shape" are superseded by this; everything else in both stands.
- A page's data now carries `period` beside the reads, as a tool's always did.
