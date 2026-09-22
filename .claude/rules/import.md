# Broker file import

- The app has a transaction file of its own (`import::canonical`, ADR-0066): `format` /
  `version` / rows in the model's own vocabulary, read by a **third reader** that flattens them
  into the same `ParsedCsv` with a fixed mapping — so a canonical file goes through the one
  wizard, the one identity check and the one commit, with nothing left to ask. Its column names
  are the canonical header aliases, so the same thing written as a CSV needs no reader at all.
  Nothing in it names an internal id: an account is its name (and `fields::account` resolves a
  name the portfolio already carries), an instrument its ticker and ISIN. `canonical_to_file`
  writes it; `transactions_export` is that with the screen's filter. Operations only — a taxonomy
  or a plan is not in it.
- A broker file is not always a CSV: `import::parse_file` picks the reader off the bytes, and an
  Interactive Brokers Flex statement (XML, ADR-0061) goes to `import::ibflex`, which flattens its
  sections into the same `ParsedCsv` and carries its own fixed `ImportMapping` — the columns are
  that module's invention, so there is nothing for a `brokers.json` preset to lay out. Everything
  after the reader is shared. Four things it resolves and nothing downstream could: an `ORDER` row
  supersedes the `EXECUTION` rows of the same purchase (asking for both prints it twice); a forex
  trade (`assetCategory="CASH"`) becomes two linked `TransferIn` legs so it is not read as leaving
  the portfolio; dates are normalised to ISO, since the query's date format is a choice in the web
  office the file never reports; and `amount_sign` is pinned to `Signed` rather than voted on. A
  contract with a multiplier other than 1, a cancelled trade and a corporate action get wordings the
  reader invents and ships in `ignored_kinds` — a split would double against already-adjusted
  quotes, and the model has no multiplier — so they stay counted and visible instead of wrong.
- Which account a file lands on is asked as **what the file is**, not as which account to pick:
  `Import/AccountTarget.tsx` offers a broker statement or a bank/wallet one, derives the cash leg
  from the choice (`Account::settlement_account_id`), and only then narrows to a specific account
  when the portfolio holds more than one of that kind. `ImportMapping::account_id` is unchanged —
  the wizard asks differently, the wire does not.
- A row's identity has a **second, looser reading**: `dedupe::loose_fingerprint` drops the amount
  and keeps account, day, kind, instrument and quantity, which is what a row edited by hand after
  it was imported still matches. Such a row is `RowStatus::Similar` and is **not** written unless
  `ImportOptions::import_similar` says so. Only a share movement has one — two cash rows differing
  in amount are two payments, and folding them would hide a real second dividend.
- An override of a field the file has **no column for** is carried beside the row
  (`cells::RowInput::added`, ahead of a rule's `emitted`), so a delivery stating only a quantity
  can be given the price it was worth. An override of a mapped field still rewrites the cell, so
  the sign and basis votes see it.
- The ISIN matches before the ticker (`preview::fields::instrument`), and a stored instrument whose
  ISIN **conflicts** with the row's is a `TickerIsinConflict` **error**, not a silent join: the
  symbol column is unique in the database, so the row waits for a ticker of its own rather than
  pouring one company's trades into another's position. `ImportService::commit` repeats the check
  against the store.
- `checks.rs` gained three heuristics and they are all warnings, as everything there is:
  `DeliveryWithoutCost` (a quantity crossing the boundary at a price of zero — the shape of a
  portfolio moved between brokers, and the most expensive thing a file can do to the numbers),
  `AccountCurrencyMismatch` (money landing where another currency is kept; asked of the
  **settlement** account and only of a row that moves money), and `PossibleSplit` (one
  instrument's prices stepping by a whole factor between two trades **within 90 days**, the
  factor exact to 1%). Both gates are the point: a split's factor is exact and instantaneous,
  while two trades a year apart in an instrument that doubled give 2.03 — which is the market.
  The reliable route to a split is the provider's own event (`events=div|split` rides the quote
  request, ADR-0034), so this covers only what that route cannot. The
  amount-vs-quantity×price allowance is per **unit** rather than flat, because a printed unit
  price is rounded and that rounding multiplies by the quantity.
- Two legs of one move that arrived from **two different exports** are never joined by the import:
  `calc::transfer_candidates` offers the pairs after the write (`transfer_suggestions`) and
  `transfer_link` joins one the user confirmed. Matching amounts is not proof, and linking the
  wrong pair erases a real deposit and a real withdrawal from every return figure at once.
- `build_preview` is pure (takes securities + fingerprints as slices); only `ImportService` touches `Store` (same rule for `commit_taxonomy`). Re-importing the same file must be a no-op — `import::fingerprint` guarantees it.
- Detection is per *language*, never per broker: a rule keyed to one broker's file helps only that broker's customers. Header aliases (`mapping::aliases`) and operation wording (`mapping::keywords`) are dictionaries and live apart from the matching that reads them (`mapping::shape`, `mapping::normalize`); both are ordered canonical-first because the index breaks ties, and sell keywords precede buy ones (`Verkoop` contains `Koop`).
- Headers lie, values do not. `ImportMapping::detect_with_values` ranks a header match exact > whole word > substring, drops a claim on a column another field names better ("Asset type" is a kind, so it is not a symbol), and lets `ValueShape` veto a weak match — a currency or ISIN column is required to look like one, a date is not, because its format may simply be unknown to us. A column that is empty in every row is not a mapping.
- A file is not necessarily UTF-8 and the user cannot be told to re-save it: `parse.rs` decodes via BOM, then UTF-8, then `chardetng`. Per-file settings (`skip_top_rows`, `skip_bottom_rows`, `date_format`, `decimal_separator`) are detected when left at zero/`None`, and the detected value is returned in `ParsedCsv::config` so the UI shows and can override it.
- One file can hold two shapes of the same thing — Trade Republic prints `2024-11-30` and `2025-01-16T16:13:36`, `-10,84` and `-24.999996`. Detection takes the majority; a disagreeing cell is parsed by any known format (warning, not error), and `parse_decimal` decides the separator from the cell, using the file's only for an ambiguous `1,234`. A cell with no letter or digit (`-`, `—`) is an absent number, not a broken one.
- Broker layouts ship with the app as data: `core/presets/brokers.json` (`import::presets`), one entry per broker, `include_str!`-ed and merged with `default_kind_aliases()` on use, so the file lists only what is peculiar to that broker. The host lists the user's own layouts first and the shipped ones after (`app/src-tauri/src/import_templates.rs`); a shipped preset is removed by writing its name down (`import_presets_hidden.json`), never by editing what ships, and `import_presets_restore` brings them all back. A user layout saved under a shipped name shadows it; deleting that copy uncovers the original.
- A layout is applied to the *next* export, not the one it was made from, so `build_preview` tops up `kind_aliases` from the keyword dictionary for every value the layout does not answer (Saxo writes the operation as `Sell 3 @ 139.74 USD` — no list enumerates that). Values already aliased or already skipped are left alone, and the topped-up mapping is what the preview hands back.
- Direction comes from the number that carries it: the amount for a cash operation, the **quantity** for a share movement, which has no cash to sign (`Reverse Split` is `+1` and `-10` of one wording). `resolve_direction` therefore compares against `cash_sign()` or, when that is zero, `quantity_sign()`.
- The amount column is gross or net of the row's own charges, and the file says which by being
  consistent: `checks::count_basis_vote` compares `quantity × price` with the amount and with the
  amount plus/minus the charges (`TransactionKind::charge_sign`), three decisive rows decide it,
  and a disagreement is a warning rather than a refusal. `Net` is put back into the model's own
  shape by `preview::fields::restore_gross` — the stored `amount` is always before charges — and
  only for the charges in the row's own currency, since a foreign one was never in that total.
  `ImportMapping::amount_basis` overrides the vote, exactly like `amount_sign`.
- A file that names its rows is recognised by that name: `ImportField::ExternalId` has
  `ValueShape::Unique` and shares its aliases with `LinkId`, whose `ValueShape::Link` demands the
  opposite — the values decide which of the two a column is, so a vetoed field yields the column
  instead of blocking it (`detect_with_values`). Same id and same fingerprint is a `Duplicate`;
  same id and different values is `RowStatus::Updated`, which **replaces** the stored row on
  commit and is counted apart from what was imported (ADR-0065).
- A file is recognised as a layout before anything is detected from it: `presets::best_match`
  scores every shipped preset and user template (`import_load`), a preset's rule being its own
  `match` block or, absent one, the columns it maps — a layout naming "Wertpapierbezeichnung"
  already describes its broker. Everything a rule declares must hold, and a tie is **no** answer:
  two layouts fitting equally well means neither was recognised. The applied name comes back as
  `applied_template`, and "— detect —" in the wizard puts the core's own reading back.
- What a row *becomes* can be a rule rather than a wording (`mapping::rules`, ADR-0067):
  conditions over **mapped fields** (`equals`, `contains`, `sign`, `present`, `empty`, joined by
  AND, first match wins) and a list of operations to emit, each setting fields to a constant or
  to `{field}` of the same row. No arithmetic and no regex — the net-amount case is
  `amount_basis`, and captures belong to column extraction, not here. An empty `emit` drops the
  row like the skip list does. The parts share the file row's `number` and carry `part`, so
  identity, problems and the commit are unchanged; a broker's id gains `#n` per part and a
  `link`ed rule gives its parts one derived link id. A rule decides direction, so `directed`
  does not flip it. The wizard offers two splits (`SPLITS` in `Import/labels.ts`) beside the
  kinds; anything else is written in the layout.
- Import is semi-automatic: everything auto-detected (`ParsedCsv::config`, `ImportPreview::mapping`) must stay overridable — parse settings, column mapping, kind/symbol aliases, per-cell `RowOverride`, `ImportMapping::amount_sign`. Never silently guess for the user.
- A broker file carries direction in two channels: the type column says *what* happened, the sign of the amount says *which way*. One type value can span both directions (e.g. a card charge and its refund), so `checks::decide_amount_sign` correlates `sign(amount)` with `TransactionKind::cash_sign` across the whole file and calls it `Signed` only when ≥90% of cash-moving rows agree **and** both signs occur; a disagreeing row flips to `TransactionKind::reversed`. Buy/Sell vote but never flip — their direction is also carried by quantity and by having a security.
- The two legs of one internal wording are linked by `build_preview` (same date, same source wording, opposite direction, link id derived from those three so the preview stays reproducible). A leg left without a partner is a one-sided transfer, which `calc` reads as money crossing the portfolio boundary — see ADR-0020.
- A `link_id` is a claim two rows make together, so it is only believed when two rows make it. A
  file's own `LinkId` column has `ValueShape::Link` and is refused — even on an exact header match,
  unlike every other shape — unless some value in it repeats: brokers print a per-row identifier
  under the same words (`Reference`, `Transaction ID`), and one id per row marks every transfer
  internal. `calc::holdings::paired_links` applies the same rule over the stored set, the way
  `scoped_transactions` always has, because that is the only place both legs are certainly visible —
  a leg whose partner is nowhere is money crossing the boundary, and counting it as internal erases
  every deposit and withdrawal from TWR, XIRR and the capital each rate divides by.
- Value that moves *inside* the portfolio — a currency exchange, a crypto conversion, a stake, a wallet-to-wallet move — is one wording on two rows (`Balance Conversion`, `USDT -> EUR`), and only the sign tells the legs apart. The keyword maps it to `TransferIn`, the reversible side, and `resolve_direction` turns the negative leg into `TransferOut`. Such a row therefore **flips but never votes** in `checks::count_vote` — the mirror of Buy/Sell, which vote but never flip. Counting it would be counting a direction we invented: half the legs of a crypto file are negative by construction, and the file would judge itself unsigned and credit both legs.
- A value the user marks "do not import" lands in `ImportMapping::ignored_kinds` and its rows get `RowStatus::Ignored` — counted in `summary.ignored`, out of `unknown_kinds()`, never written. A broker prints lines that are not operations (`Name Change`, `Monthly statement`); refusing the file over them is not an answer, and neither is importing them as something else. The choice is exclusive with a kind alias and stays visible in `preview.kinds` (`ignored: true`) so it can be taken back.
- Row problems carry a `Severity` and a `ProblemCode`. Only `Error` makes a row `Invalid` — a direction-corrected row is `summary.warnings`, not `summary.invalid`. `checks::check_row`/`check_file` are heuristics and only ever emit warnings; a false positive must not block an import.
- Taxonomy CSV is read by meaning, not template: level columns found by header name (`Levels 2`, `Уровень 2`, `Category`), a security row identified by having a ticker/ISIN, the security's own name one level deeper than its category. The first level (tree's name, repeated every row) is detected and dropped. Securities are matched by ISIN first (ISIN = instrument, ticker = listing — a foreign file may print a different one). `commit_taxonomy(into)` extends the tree it's invoked on; a node already present under the same name/place is reused (case-insensitive match), so re-importing doesn't double the tree, and a security's split is overwritten by the newer file.