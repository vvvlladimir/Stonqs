---
paths:
  - "core/src/calc/allocation/**"
  - "core/src/calc/rebalance/**"
  - "core/src/calc/breakdown.rs"
  - "core/src/import/taxonomy/**"
  - "core/src/model/taxonomy*"
  - "core/src/storage/taxonomies.rs"
  - "core/src/storage/targets.rs"
  - "app/src-tauri/src/commands/allocation.rs"
  - "app/src-tauri/src/ai/tools/allocation.rs"
  - "app/src-tauri/src/ai/tools/taxonomy.rs"
  - "app/src/screens/Allocation/**"
  - "app/src/screens/Rebalance/**"
---

# Taxonomy, allocation, rebalance

- Subjects (`TaxonomySubject`): a position (key = `security_id`) or cash (key = `cash:<account_id>:<CUR>`, `model::cash_subject_key`) — account **and** currency. `allocation_by_taxonomy` divides by the sum of *included* subjects.
- Storage: `security_classifications` + `cash_classifications`; calc and wire see one `Assignment { subject_id, node_id, weight }`. Maths never branches on security vs cash.
- Seeding from an attribute (`import::group_by_attribute`, ADR-0032): one node per distinct `TEXT` value, assigned whole; plan is a `TaxonomyPreview` written by `commit_taxonomy`. Already classified → planned, not assigned (`matched_by: "already_classified"`); missing value → no node; number/date refused. `taxonomy_group_commit` with `into: null` creates the tree. `TaxonomyInput::kind` optional (absent keeps / new = `Custom`).
- Plugin taxonomies: `TaxonomyDef { id, name, file }` (a CSV) fed through `taxonomy_import_preview`/`_commit` via `plugin_taxonomy_csv`; no expectation shipped; `plugins::verify::taxonomy` checks it reads as a tree against an **empty** securities list.
- Exclusion is per taxonomy and freezes (`taxonomy_exclusions`, `subject_id` has no FK since cash isn't a row): an excluded subject leaves denominator/chart/percentages entirely (not moved to `allocation::UNCLASSIFIED_KEY`), still listed by `allocation_members` (`excluded: true`, `weight: 0`, last). Classification untouched.
- `TargetWeight::weight` is a share of its **parent**; absolute = product along the path (`AllocationTarget::absolute_weights`), never stored. Only `leaf_node_ids` divide money; a weighted parent gets a row/drift, no trades, no share of `off_target_base`. `validate_tree` (from `Store::save_target`) checks each sibling set.
- `rebalance` spreads a node's drift over all its subjects: security → `RebalanceTrade`; cash → `CashDeposit` (no price/quantity/step).
- `cash_to_invest` is added to the total **before** targets. `allow_sell = false`: overweights untouched, new money split across underweights proportional to shortfall. `calc::rebalance::spend_leftover` then adds one step at a time (largest shortfall first, then least overweight), never touches a node with nothing to buy, never sells; remainder below the cheapest unit stays in `cash_left_base`.
