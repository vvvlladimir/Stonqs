# 79: A latest-close source extends the tail and never rewrites the history

- Status: Accepted

## Context

Some instruments are quoted late by the source that holds their best history: a fund's net asset
value arrives a day or two after the fact, a thin venue publishes after the close. Portfolio
Performance answers this with a second feed per instrument — history from one, the latest price
from another. Here an instrument already records its symbol at other sources (`security_symbols`,
ADR-0052), but those are asked only when the own source *fails*; a source that answers late does
not fail, so the last days simply stay empty and the portfolio is valued at an old close.

Quotes are daily closes keyed by date (`PricePoint`), so "latest price" means the most recent
closes, not an intraday tick.

## Decision

- An instrument may name **one** of its other sources as the latest-close source: a `latest` flag
  on the `security_symbols` row (migration 0030), with a partial unique index so at most one row
  per instrument carries it. The role lives on the row that already holds the symbol; forgetting
  the symbol forgets the role, and switching the listing (which clears `security_symbols`) clears
  it too.
- `MarketDataService::ensure_latest` runs after the history is ensured. It asks that source only
  when the stored series ends before today, and only for the window from 14 days before the last
  stored close through today. What it returns passes `market::guard::check` against the stored
  overlap (same currency, median ratio within 2%) or is refused whole.
- Only days **after** the last stored close are written, with `INSERT OR IGNORE`, and no coverage
  is extended. A hole inside the history stays the fallbacks' business (ADR-0052).
- A failure is reported as a quote failure named after the latest-close source, and leaves the
  history untouched.

## Alternatives

- **A `role` column on `security_symbols` with FALLBACK/LATEST.** The primary key is
  `(security_id, source)`, so one source could not be both; a flag keeps every symbol a fallback
  and adds the role on top.
- **A `latest_source` column on `securities`.** Every `Security` literal and the wire shape would
  change for a fact that only the quote chain reads, and the symbol would still live in the other
  table.
- **Let the latest source upsert.** It would overwrite the own source's closes on the days both
  hold, so the chosen history would stop being the chosen history.

## Consequences

- A day the own source is later asked for and answers replaces the close written here. Coverage
  runs through yesterday, so a day the own source was already asked for and did not publish keeps
  the latest source's close — which is the point: that day would otherwise have none.
- On a weekend the series ends on Friday, so each refresh asks the latest source once per such
  instrument and writes nothing new.
- The meaning of every figure is unchanged; only where the last closes came from differs.
