# Instruments, listings and quotes

An **ISIN identifies an instrument**; a **ticker identifies a listing**. The same ETF trades in
London, Amsterdam and Frankfurt under three tickers, in three currencies, with three price series.
The user picks which listing their holding is quoted from — the app can suggest, but the choice is
theirs, and a suggestion preferring the base currency is a preference, not a fact.

Consequences worth knowing:

- An ISIN is never a quote symbol. An instrument identified only by ISIN has no prices until a
  listing is chosen for it.
- Switching an instrument to a different listing drops the price history stored under the old
  symbol, because that series belonged to that venue. Naming the venue of a listing already in use
  changes nothing and drops nothing.
- Many instruments have no ISIN at all — US stocks as most providers report them. That is normal,
  not a data problem to fix.

**A source answering with almost nothing is a venue fault, not an empty market.** A ticker taken
from a broker's file often belongs to an exchange the price source does not index. The source then
answers the request rather than failing it, but returns only today's close, so the instrument shows
a price and no history at all. The app calls a series that covers under a quarter of the period the
instrument has been held too short, flags it, and — for an instrument it chose the venue for
itself — moves it to another exchange that really has prices and fetches it there. A holding
younger than about three months is never judged this way, and an instrument that simply listed
part-way through the period is not either: a series short because the listing is young is a fact,
not a fault.

**How far back prices are fetched is decided by the ledger.** An automatic fetch covers each
instrument from its first operation and each currency pair from the first operation using it, so
importing history older than the app's default window does not leave that older part unvalued.
A manual refresh of recent prices only extends what is already stored and never closes a hole
older than the series.

An instrument can also be set to manual prices, in which case no provider is asked and the user
enters closes themselves. Missing quotes on such an instrument are expected.

Quote currency is the listing's, and it is not the currency the position was paid in. See the
currency topic before multiplying anything by a rate.
