# 53: Market sources are switched and keyed by the user

- Status: Accepted

## Context

ADR-0050 made the shipped sources a catalogue, but every row was either on for everyone or off
for everyone, and none could take a key. The sources worth adding next — Twelve Data (stocks, ETFs,
FX, crypto on one endpoint) and EODHD (end-of-day prices for most exchanges) — answer nothing
without a key, and a user may reasonably want Yahoo off altogether.

## Decision

- The core describes, the host decides. `sq_core::sources::Setup { keys, switched }` is what the
  user chose; `quote_service_with(&Setup)` / `fx_service_with(&Setup)` build from it, and a row with
  `KeyUse::Required` and no key is not built at all. A row's constructor takes `Option<String>`.
- Keys live in the profile vault under `market:<source id>` (ADR-0048), beside and apart from the
  AI providers' keys, through the same `key_save` / `key_for_call`. No command reads one back;
  `market_sources_list` reports `has_key`.
- Switches live in `AppSettings::market_sources` (source id -> on), storing only what differs
  from the default, with their own command (`market_source_switch`); `settings_save` preserves
  them. It is host state because the host decides what leaves the machine.
- `AppState::market_setup()` copies both out before any network work, so a refresh or a lookup
  never holds the vault or settings lock across a request. A locked profile yields no keys.
- A provider that reports errors inside a 200 (Twelve Data) maps its body `code` onto the same
  `Unauthorized` / `RateLimited` / `Unavailable` a status code would give.

## Alternatives

- **Keys in `settings.json`.** Plain text beside the database; ADR-0048 already rejected that.
- **One global switch per role (quotes/FX).** Too coarse: the choice is per source.

## Consequences

- Twelve Data joins the FX chain after the keyless sources; for quotes it and EODHD are chosen per
  instrument or asked as fallbacks through `security_symbols`.
- A source's per-day budget is not tracked yet; a throttled source rests through the breaker
  (ADR-0052) rather than being paced.
