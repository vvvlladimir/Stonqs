# Valuation and prices

A valuation is built in one order, always: transactions produce holdings, holdings are priced,
metrics are computed on the result. Holdings themselves never touch market data — how many shares
are held on a date is a fact about the ledger, and it is the same whether or not a price exists.

Prices are read **as of a date, or the last known before it**, never forward in time. A Saturday is
valued at Friday's close; a suspended instrument keeps its last close until a new one arrives. That
is why a value can be stale without being wrong, and why a portfolio has a value on every calendar
day while risk figures are computed on trading days only.

A missing price is not zero and not silently skipped: it is an error that names the instrument and
the date. If a report cannot be produced because a price is missing, say which instrument is
missing prices instead of reporting a smaller total.

Prices are stored already adjusted for splits. A historical close therefore matches today's share
count, and a split is reflected in the price series without any correction on top of it.

A price always travels with the currency it is quoted in, which is the *listing's* currency and not
necessarily the currency the user paid in. Those two being different is normal for European
investors: a US-listed instrument quoted in USD, bought from a EUR account.

Each instrument has its own price source. When that source cannot be reached, and the instrument
also has a symbol at another source, the other source fills the missing days — but only if its
closes agree with the stored ones within two percent and are in the same currency. A source that
disagrees (a different split adjustment, pence instead of pounds) is ignored rather than mixed in.
Filled days are temporary: the next refresh asks the instrument's own source again and replaces
them. A source that fails several times in a row is skipped for the rest of that refresh.
