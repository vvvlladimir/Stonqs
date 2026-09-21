# 32: A taxonomy is seeded from an attribute, never derived from it

- Status: Accepted

## Context

ADR-0031 gave instruments attributes the user invents. The obvious next question is whether a
taxonomy is one of them: "Country = Ireland" reads exactly like a flat tree with a node per value,
and some trackers do build taxonomies out of attributes.

They are not the same thing. A taxonomy splits a subject's *weight*: IWDA is 70% United States,
12% Japan and so on, one instrument in several nodes (`Assignment { subject_id, node_id, weight }`).
An attribute holds one value. A taxonomy is a tree whose weights are relative to the parent; an
attribute has no parent. A taxonomy classifies cash as well (`cash:<account_id>:<CUR>`), and a cash
balance has no ISIN to hang an attribute on. Targets, drift, exclusions and colours are properties
of a node. In the other direction, an attribute holds numbers and dates, and a tree of distinct TER
values is a list, not a classification.

## Decision

The bridge goes one way: an attribute **seeds** a tree, the tree stays the source of truth.

`import::group_by_attribute` plans one node per distinct value of a text attribute, with every
instrument carrying that value assigned whole (weight 1). It returns the same `TaxonomyPreview` the
CSV import produces and is committed by the same `commit_taxonomy`, so node reuse ("a node under the
same name in the same place is reused") and target handling have one implementation.

Three rules make a second run safe:

- **What the tree already classifies is left alone.** Such an instrument is still planned, with
  `security_id: None` and `matched_by: "already_classified"`, so the dialog can count it; nothing is
  written for it. A 60/40 split typed by hand outranks a value read off a column.
- **A missing value is not a group.** The instrument stays unclassified, which `allocation` already
  reports through `UNCLASSIFIED_KEY`. No "Unknown" node is invented.
- **Only a `TEXT` attribute groups.** A node per distinct number or date would put every instrument
  in a group of its own; grouping dates by year or numbers into bands is a different feature, and
  the core refuses the call rather than producing that.

Groups are ordered by size, largest first: node order is what the charts colour by.

## Alternatives

- **Derive the tree from attributes live.** Then a single instrument's split could never be
  overridden by hand, and cash — which has no attributes — would drop out of the classification
  entirely.
- **Store classification as an attribute.** Loses weights, the tree, targets, exclusions and cash
  subjects.
- **An attribute kind that points at a node.** The same thing by a longer road: half of the tree's
  semantics would move into the value table without the other half.
- **Move the instrument when its value changes.** Silently undoing the user's own edit. The plan
  reports the divergence instead, and the user decides.

## Consequences

- Grouping is idempotent in practice: running it again after filling in more instruments adds only
  what is new.
- The preview is a plain read, so the dialog can show what would happen (categories, instruments,
  how many are left alone) before anything is written.
- `calc` is untouched: grouping writes the same classifications a hand assignment writes.
- Grouping plans no targets — a target is set on the tree itself, as before.
- If grouping by a number or a date is ever wanted, it needs a bucketing rule (years, bands), and
  that rule is the decision, not the storage.
