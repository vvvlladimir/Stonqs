# 52: A fallback quote source fills gaps and never rewrites the series

- Status: Accepted

## Context

An instrument had exactly one quote source (`securities.data_source`). When it failed — Yahoo
throttling, an outage, a ticker it dropped — the instrument simply had no new prices. Asking a
second source is the obvious cure and a dangerous one: two sources rarely quote the same series.
One adjusts for dividends and the other does not, one publishes a split a day late, one quotes
London in pence. Stitching their closes together produces a chart with jumps that are not in the
market and returns that were never earned.

## Decision

- An instrument's symbol at *other* sources lives in `security_symbols` (migration 0023). Its own
  source keeps reading `data_symbol`/`symbol`; `Store::symbol_at` answers for both. A source with no
  symbol for an instrument is never asked for it.
- `MarketDataService::ensure_history_through` asks the own source first. Only when it **fails**
  are the others asked, in registration (catalogue) order, and only those that `covers` the
  instrument.
- A fallback is asked from 14 days before the gap so its closes overlap the stored series.
  `market::guard::check` refuses it unless it is in the series' currency and its median ratio to
  the stored closes on common days is within 2%.
- An accepted fallback writes with `INSERT OR IGNORE` (`Store::fill_quotes`), and **coverage is not
  extended**: the next refresh asks the own source for the same gap again, and its upsert replaces
  the filled days. A fallback is a bridge, not a new owner.
- Every call goes through a breaker: a source failing `RESTS_AFTER` (3) times in a row, or once
  with a rejected key, is skipped for the rest of that service's life (one refresh).

## Alternatives

- **Fall back on an empty answer too.** An empty range is usually a holiday; asking on would mix
  sources on every long weekend.
- **Adopt the fallback's series as the new own source.** Rewrites history silently; the user chose
  the source, and a transient outage must not undo that.
- **Adjust the fallback to the stored series (scale by the ratio).** Hides a real disagreement and
  invents prices nobody quoted.

## Consequences

- With one quote source registered, nothing changes; the chain starts to matter as sources with
  per-instrument symbols are added.
- A refresh against a dead source ends in three requests instead of one per instrument.
- A days-long gap bridged by a fallback shows that source's name on those quotes until the own
  source answers again.
