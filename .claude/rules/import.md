---
paths:
  - "core/src/import/**"
  - "core/presets/**"
  - "core/tests/phase3_import/**"
  - "core/tests/fixtures/presets/**"
  - "core/tests/attribute_csv.rs"
  - "core/src/calc/transfers.rs"
  - "fuzz/**"
  - "app/src-tauri/src/commands/import.rs"
  - "app/src-tauri/src/import_templates.rs"
  - "app/src-tauri/tests/wizard_fixture.rs"
  - "app/src/screens/Import/**"
---

# Broker file import

## Readers
- `import::parse_file` picks the reader from the bytes: canonical (`import::canonical`, ADR-0066), IB Flex XML (`import::ibflex`, ADR-0061), plugin readers (host-side, see `plugins.md`), then CSV. All flatten into `ParsedCsv`; everything after is shared.
- Canonical: `format`/`version`/rows in model vocabulary; column names = canonical header aliases (so a CSV with them needs no reader). No internal ids — accounts by name, instruments by ticker+ISIN. `canonical_to_file` writes; `transactions_export` = that + the screen filter. Operations only.
- IB Flex: fixed `ImportMapping` in canonical column names. `ORDER` supersedes its `EXECUTION` rows; forex (`assetCategory="CASH"`) → two linked `TransferIn` legs; dates → ISO; `amount_sign` pinned `Signed`. Multiplier ≠ 1, cancelled trades, corporate actions → reader-invented wordings in `ignored_kinds`.
- Encoding: BOM → UTF-8 → `chardetng`. `skip_top_rows`, `skip_bottom_rows`, `date_format`, `decimal_separator` detected when zero/`None` and returned in `ParsedCsv::config`. Mixed formats in one file: majority wins, stragglers parsed by any known format (warning). `parse_decimal` decides separator per cell (file's only for ambiguous `1,234`). A cell with no letter/digit (`-`, `—`) is absent.

## Mapping and detection
- Detection per *language*, never per broker. Dictionaries `mapping::aliases` (headers) and `mapping::keywords` (wordings) are separate from matching (`mapping::shape`, `mapping::normalize`); canonical-first order breaks ties; sell before buy (`Verkoop` ⊃ `Koop`).
- `ImportMapping::detect_with_values`: exact > whole word > substring; drops a claim a better field makes; `ValueShape` vetoes weak matches (currency/ISIN must look like one; dates not). All-empty column = no mapping.
- `ExternalId` (`ValueShape::Unique`) and `LinkId` (`ValueShape::Link`) share aliases; values decide. `LinkId` is refused even on exact header match unless some value repeats.
- Layout recognition first: `presets::best_match` scores presets + user templates (a preset's `match` block, else its mapped columns); everything must hold; a tie = no answer. Returned as `applied_template`.
- Presets: `core/presets/brokers.json`, `include_str!`-ed, merged with `default_kind_aliases()`; list only broker peculiarities. User layouts first; hiding a shipped one writes `import_presets_hidden.json` (`import_presets_restore`); same-name user layout shadows.
- `build_preview` tops up `kind_aliases` from keywords for unanswered values (already aliased/skipped left alone) and returns the topped-up mapping.
- Rules (`mapping::rules`, ADR-0067): conditions over mapped fields (`equals`, `contains`, `sign`, `present`, `empty`; AND; first match), emit ops with constants or `{field}`. No arithmetic/regex. Empty `emit` drops. Parts share `number`, carry `part`; external id gains `#n`; `link`ed rule → one derived link id. A rule fixes direction. Wizard offers `SPLITS` (`Import/labels.ts`).
- Everything detected stays overridable (config, mapping, aliases, `RowOverride`, `amount_sign`, `amount_basis`). Never silently guess.
- Override of an unmapped field rides in `cells::RowInput::added` (before a rule's `emitted`); of a mapped field rewrites the cell (so votes see it).
- `ignored_kinds` → `RowStatus::Ignored`, counted in `summary.ignored`, exclusive with a kind alias, visible in `preview.kinds` (`ignored: true`).

## Direction, sign, basis
- Direction from the carrying number: amount for cash ops, **quantity** for share movements (`resolve_direction`: `cash_sign()` or, if zero, `quantity_sign()`).
- `checks::decide_amount_sign`: `Signed` only if ≥90% of cash rows agree **and** both signs occur; disagreeing rows flip to `TransactionKind::reversed`. Buy/Sell vote, never flip. Internal-value wordings map to `TransferIn`; negative leg → `TransferOut`; they flip but **never vote** (`checks::count_vote`).
- Gross vs net: `checks::count_basis_vote` (3 decisive rows; disagreement = warning). `Net` restored by `preview::fields::restore_gross` for same-currency charges only. Stored `amount` is always before charges.
- Same-file internal legs linked by `build_preview` (same date, wording, opposite direction; derived link id). Unpaired leg = boundary crossing (ADR-0020). `calc::holdings::paired_links` re-checks over stored rows.
- Legs from two exports are never auto-joined: `calc::transfer_candidates` → `transfer_suggestions`, user confirms `transfer_link`.

## Identity and checks
- `build_preview` is pure (securities + fingerprints as slices); only `ImportService` touches `Store` (same for `commit_taxonomy`). Re-import = no-op (`import::fingerprint`).
- Same external id + same fingerprint = `Duplicate`; different values = `RowStatus::Updated`, replaces on commit, counted apart (ADR-0065).
- `dedupe::loose_fingerprint` (no amount; share movements only) → `RowStatus::Similar`, written only with `ImportOptions::import_similar`.
- ISIN before ticker (`preview::fields::instrument`); conflicting ISIN on a stored ticker = `TickerIsinConflict` **error**, re-checked in `ImportService::commit`.
- Only `Severity::Error` makes a row `Invalid`. `checks::check_row`/`check_file` emit warnings only: `DeliveryWithoutCost`, `AccountCurrencyMismatch` (settlement account, money-moving rows), `PossibleSplit` (within 90 days, factor exact to 1%). Amount-vs-qty×price tolerance is per unit.
- Disposal checked against stored ledger + file rows (`preview::holdings`, `calc::quantity_gaps`): `SaleExceedsHoldings` warning naming `unread`/`unread_kinds` (ADR-0089). Identity counted as a multiset.
- Account target is asked as what the file is (`Import/AccountTarget.tsx`: broker vs bank statement; cash leg from `Account::settlement_account_id`). Wire (`ImportMapping::account_id`) unchanged.

## Side CSVs
- Attributes (`import::attributes`): row per instrument, column per attribute; pure `build_attribute_preview`, `commit_attributes`; matched via `taxonomy::match_security`. Existing column keeps its kind (bad cell = error on that cell only); new column's kind inferred. Blank = absent (merge). Idempotent.
- Taxonomy CSV: read by meaning — level columns by header, security row = has ticker/ISIN, first level (tree name) dropped, ISIN before ticker. `commit_taxonomy(into)` reuses nodes (case-insensitive); newer file overwrites a security's split.

## Tests
- `core/tests/phase3_import/` (one binary): worked examples plus `robustness.rs` (hostile input must *answer* — error or row problem, never panic or silent loss), `properties.rs` (proptest: determinism, order independence, idempotence, canonical round trip), `synthetic.rs` (each preset over a file from its own declaration), `perf.rs` (ignored, release).
- `conformance.rs` + `core/tests/fixtures/presets/<layout>/` (redacted export, expected recognition, expected ops in canonical format). `UPDATE_FIXTURES=1` regenerates then fails on purpose. Every preset must match its own header; fixture-less presets are a named list.
- `fuzz/`: libFuzzer over `parse_file`, `parse_canonical`, `parse_flex`, preview; crashes become cases in `robustness.rs`.
- Frontend: `screens/Import/model.ts` (+`model.test.ts`); `wizard.dom.test.tsx` driven by `app/e2e/fixtures/import-wizard.json` generated by `app/src-tauri/tests/wizard_fixture.rs`.
