# Instruments

The directory of everything the portfolio knows about: tickers, names, ISINs, where each one's
prices come from, and the user's own notes and attributes. It is not scoped by the account picker —
an instrument is an instrument.

**An instrument and a listing are not the same thing.** One ISIN trades on several exchanges under
several tickers, in different currencies. The row's *venue* is which listing this instrument's
prices are read from; the Venues action lists what the directory knows and lets the user choose.
Changing to a genuinely different listing throws away the price history stored under the old symbol,
because that series belonged to the old symbol — naming the venue of a listing already in use
deletes nothing.

**Two faults have their own slices** in the filter strip, because they are different problems:

- *Not identified* — the row carries an ISIN where the quote provider expects a ticker, which is
  what a fresh import looks like. "Identify" looks the instrument up and fills in the real symbol;
  it can be done for all of them at once, one after another.
- *No venue* — prices are being read without it being stated which exchange they come from.

A banner also warns when an instrument's quote currency differs from the currency recorded for it.
That usually means the prices are coming from another listing than the one the user thinks.

**A quote history shown in red is too short for how long the instrument has been held.** The source
answered — often with today's price alone — but the series covers a sliver of the period the
instrument has been owned, so charts and past values of that position have nothing to read. It is
almost always a ticker belonging to an exchange this source does not quote. The app repairs this by
itself for an instrument it added or looked up on its own: the next time it fetches what is
missing, it searches the exchanges the instrument trades on, moves it to the first one that really
has prices, and fetches the history there. Where no such exchange is found, the row stays red and
Venues is the manual way out. Instruments in this state are counted in the line above the table.

**Prices come from a provider or by hand.** An instrument with no data source is priced manually
and is never touched by a refresh; a new instrument defaults to being quoted. "Refresh quotes"
fetches what is missing for every instrument, and the app also fetches quotes by itself whenever
something new appears — an import, a new instrument, a new transaction. That automatic fetch
reaches back to the instrument's first operation rather than to a fixed number of years, so
importing a decade of history does not leave the older half of it unpriced.

Each instrument carries a note, a WKN, a tradable quantity step (which is what rebalancing rounds
to), and any attributes the user has defined — an attribute can also seed a whole classification
tree from its values.

**Splits** are edited per instrument. A split adjusts the lots held, never the stored quotes:
quotes arrive already adjusted from the provider.

An instrument used by transactions cannot be deleted, and the menu says why rather than failing on
the press.
