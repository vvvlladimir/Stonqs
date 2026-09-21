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

An instrument can also be set to manual prices, in which case no provider is asked and the user
enters closes themselves. Missing quotes on such an instrument are expected.

Quote currency is the listing's, and it is not the currency the position was paid in. See the
currency topic before multiplying anything by a rate.
