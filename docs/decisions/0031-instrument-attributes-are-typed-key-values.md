# 31: Instrument attributes are typed key/values, not columns

- Status: Accepted

## Context

An instrument carries facts no quote provider supplies: the TER of a fund, the country whose risk
it really holds, the replication method, the date the user first looked at it, why it is held at
all. Other trackers let the user define such attributes and use them as directory columns
and as the input of taxonomy auto-classification. We had none of it — not even a note, and not
the WKN that German brokers print beside the ISIN.

Two facts are different in kind from the rest. A note and a WKN are properties of every instrument
and of no other thing, and their names are ours. Everything else is invented by the user: we cannot
know that "TER" exists, let alone that it is a percentage.

## Decision

`securities` gains two columns, `wkn` and `note` (migration `0012_security_attributes.sql`).
They are ordinary `Security` fields.

Everything the user invents lives in two tables: `security_attribute_defs` (id, name, kind, unit,
position) and `security_attributes` (security_id, attribute_id, value), the value always `TEXT`.
The name is user data, seeded by nobody and never translated; values are keyed by attribute **id**,
so renaming an attribute keeps what was filled in.

`AttributeKind` is `TEXT | NUMBER | DATE` and owns `normalize`, which is what a value passes through
before it is stored: a number loses its trailing zeros, a date keeps the sortable `YYYY-MM-DD` form,
text is trimmed. There is deliberately **no percent kind** — it would have to decide between `0.07`
and `7` for every reader, and nothing computes with attributes yet. A rate is a `NUMBER` whose unit
is `%`, and the unit is shown, never parsed.

The kind is fixed once the attribute exists. Changing it would either strand values it can no longer
read or delete them silently; `attribute_def_save` therefore takes the name, the unit and the order
of an existing attribute and keeps its stored kind.

`Store::set_security_attributes` replaces every value of one instrument in one transaction, and a
blank clears the attribute instead of storing an empty string, so "not filled in" has one spelling.
On the wire, `SecurityInput::attributes` is an `Option`: absent means the caller is not editing
attributes at all, where an empty map would clear every one of them.

## Alternatives

- **A column per attribute**, added by migration. Every user invention becomes a schema change; the
  set of attributes is user data, not schema.
- **One JSON blob on `securities`**, like `AppSettings::ui`. A blob cannot answer "which instruments
  have no TER" and has no place to keep the attribute's own name, kind and order once no instrument
  uses it yet.
- **Typed value columns** (`value_text`, `value_number`, `value_date`). Money and dates are already
  stored as `TEXT` everywhere in this database (ADR-0013); a second convention for attributes would
  buy nothing while the values are only ever read back.
- **Attributes drive taxonomy classification.** Out of scope here: the
  storage below is what auto-classification would need, and it can be added without moving it.

## Consequences

- A new attribute is data, not code: no migration, no Rust change, no wire change.
- The values travel on `SecurityRow` as a `{ attribute_id: value }` map, so the directory list stays
  one query and the form offers every attribute the user defined.
- Deleting an attribute deletes its values (`ON DELETE CASCADE`) — the column is gone, not emptied.
- Nothing in `calc` reads attributes. When something does — auto-classification, a filter, a column
  that sorts — the reader will have to decide what a `NUMBER` means, and that decision gets its own
  ADR.
