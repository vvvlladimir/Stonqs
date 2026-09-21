# 35: A watchlist is a named list of instruments, read off their own quotes

- Status: Accepted

## Context

Other trackers keep watchlists: named lists of securities the user follows without
holding them, with the latest quote and its change beside each one. We want the same, with every
column the positions table offers — a watched instrument may also be held — plus what only a
watchlist asks: the period's move and range, the reported dividend, the nearest alert level.

Some of this already existed. The refresh job fetches quotes and events for every instrument in
the directory, held or not (ADR-0014, ADR-0034); alerts and events are facts about an instrument
and are not scoped; the instrument card opens for any id.

## Decision

**A watchlist is stored, and holds nothing but order.** `watchlists (id, name)` plus
`watchlist_items (watchlist_id, security_id, position)`. A watched instrument is an ordinary
`Security`: there is no "watch-only" kind, and deleting the instrument takes it off every list by
cascade. Lists read in creation order; `save_watchlist` replaces a list's items in one transaction,
so adding, removing and reordering are the same write.

**Its figures are the instrument's own.** `calc::instrument_move` reads one quote series, in the
currency of the latest close, and returns the last price, the day's change, the period's return,
low, high and where the price sits between them, the all-time high, and the dividends per share
the provider reported over the trailing year with their yield on the price. No position and no
exchange rate enter it. A period starts at the close in force on its first day; an instrument
younger than the period starts at its first close and the row says from when, as a benchmark does
(`money-and-fx.md`). A close in another currency than the latest is left out rather than divided.
`calc::nearest_level` picks the price rule with the smallest absolute distance from
`alert_status`.

**Not scoped.** `watchlist_rows` takes no source: a price does not depend on the account picker,
and neither does the list. The figures of a *position* in a watched instrument are the positions
screen's — `positions_at` and `position_returns` under the picker — joined on the frontend by
security id, so one query serves both screens and no holding arithmetic is repeated.

**Columns are chosen like the positions table's.** The positions column catalogue moves to
`components/domain/positionColumns.tsx`; the watchlist offers its quote columns, the holding
columns of that catalogue (a dash where nothing is held), the record's codes and the user's
attributes. The choice is `UiState::watch_columns`, beside `position_columns`, and
`ColumnPicker` becomes a domain-free primitive both screens feed.

**Adding an instrument that is not in the directory creates it.** The add dialog searches the
directory first, then the provider; picking a result saves a `Security` with the provider's
symbol and currency, puts it on the list and starts a catch-up refresh, so the row fills in. A
result whose currency the provider does not report is refused, not guessed.

## Alternatives

- **A flag on `securities`.** Rejected: one list only, no name and no order, and "watched" would
  become a property of the instrument rather than of the user's attention.
- **Computing the rows from `positions_at`.** Rejected: an instrument nobody holds has no
  position, which is the whole point of the screen.
- **Values in base currency.** Rejected: a watchlist compares instruments on their own terms; a
  currency move would read as the instrument's. The holding columns keep base currency, where it
  is the portfolio's money.
- **Scoping the list to a portfolio.** Rejected for now: securities are global, and a list of
  them is too.

## Consequences

- One migration (`0017_watchlists.sql`), four commands and one `data:changed` kind (`watchlists`).
- A fresh quote, a new alert or a reported event invalidates the rows through the `alerts` group.
- The dashboard gains a `watchlist` widget with its own `watchlist` field; it offers no source.
- Instruments added from the dialog appear in the directory like any other.
