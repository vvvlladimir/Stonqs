# Watchlist

Named lists of instruments to follow. Watching is not owning: an instrument can be on a list
without a single share being held, and the list is not affected by the account picker.

Every row is read from the instrument's **own quotes, in its own quote currency** — the last close,
the day's move, the move over the chosen period, the range it traded in. These are prices, not
positions: nothing here is converted to base currency or weighted by what is held.

When an instrument *is* held, the row can also show what that position is doing — its value,
weight, unrealised result, the period's return. Those figures come from the positions screen under
the current account picker and are joined in, never recomputed here, so they always agree with what
Positions says.

**Columns are the user's choice**, from the same catalogue the positions table uses, and the two
screens remember their choices, their order and their sorting separately. They are switched on,
reordered by dragging and searched exactly as on Positions.

Several columns come in pairs that read alike and mean different things, which is why the picker
groups them by source. What the data source *reported* about the instrument — `Dividend a year`,
`Dividend yield`, `Last ex-date` — is per share and exists whether anything is held or not. What
the portfolio *received* — `Dividends received`, `Yield received`, `Last received` — is money this
portfolio was actually paid, and is empty for an instrument nobody holds. `Period high` and
`Period low` are the chosen window; `All-time high` is the highest close stored for that instrument
whatever the window.

The period control sets the window the move is measured over. It defaults to a year, because a
watched instrument has no inception in this portfolio to measure "since the beginning" from.

Lists are created, renamed and deleted from this screen, instruments are added from the directory,
and the order inside a list is the user's — rows move up and down by hand. Clicking a column
heading sorts the table on top of that; clicking it a third time gives the hand-made order back.

A row also shows the price alert its instrument is closest to, if there is one, which is what makes
the list readable as "how near is anything to the level I care about".
