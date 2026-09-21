# Watchlists

A watchlist is a list of instruments to follow, held or not. Deleting an instrument from the
portfolio takes it off every list; saving a list replaces its contents.

A watchlist row is read from the instrument's **own** price series in its **own** quote currency —
its move over the period is the listing's move, with no conversion and no position behind it. The
same instrument's row in the positions table is a different number: there it is converted to base
currency, weighted and scoped.

So when a watchlist and the positions table disagree about a percentage, both are right and they are
answering different questions: one is the instrument, the other is the user's holding of it.
