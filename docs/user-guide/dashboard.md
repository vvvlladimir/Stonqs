# Dashboard

A board of tiles the user arranges themselves. Nothing here is fixed: the shipped board is a
starting point, and a user who has rearranged it is not looking at the same screen as one who has
not — never describe a tile as being "in the top right" or "on the dashboard by default".

**Tiles follow both lenses.** A tile reads the accounts in view unless it names a data source of
its own, and every tile is read at the date the app is set to — a board looked at with the date
moved back is that board as it stood then.

**Several boards.** The tabs across the controls switch between boards; each keeps its own tiles.
A board can be renamed, duplicated, deleted, exported to a file and imported from one — which is
also how the shipped board was authored.

**Edit mode** is a toggle. Outside it the tiles are just read; inside it, any edge or corner
resizes a tile, its header moves it, and the tile being moved follows the pointer while a
placeholder shows where it would land. The edge being dragged is the one that moves: shrinking a
tile from its left or top edge leaves empty space on that side, and the tile can grow back into
that space — but not past it, into a tile beside or above it. Moving a tile drops that empty
space and lets it sit wherever it lands. Widths are in twelfths of the grid and fold down to fewer
columns on a narrow window, so one layout serves every width rather than one per breakpoint.

**The period control at the top is the board's**, shared by every tile that reports over a window.
A tile may override it with a period of its own, and then the board's control does not move it.

**Each tile configures itself.** Every tile that reads data offers a *data source* — its own
account scope — so one board can hold a tile per account without the picker around it changing.
Left empty, a tile follows the account picker. Depending on the tile, its dialog also offers a
period, a classification tree, a benchmark, a target, how many rows to show, and a title of the
user's own.

Every tile also offers a **smallest width**, in twelfths of the grid. It is where a drag stops, and
it is what decides how the tile behaves on a phone: a tile asking for four twelfths or more takes a
whole row there instead of half of one, which is how a chart avoids being drawn 150 pixels wide.
Left at the number already in the field, the tile keeps the width the app thinks it reads at. The
tile's own width is raised to this if it was smaller.

**What can be placed**, from the palette:

- *Numbers* — a single metric from a catalog (value, return, drawdown and so on), with an optional
  footnote showing the period's dates or name; a *Ratio* of any two of those figures, shown as a
  percentage or as a multiple; and *Progress*, one tile for the three things that are a figure over
  a track. Its *What it tracks* setting picks which: a savings *Goal*, a *Contribution limit*, or
  *Financial independence*, which reads a target off the spending it is told to cover. Only the
  chosen subject's own settings are then asked for, and the tile titles itself after it.
- *Charts* — value and flows, portfolio against a benchmark, drawdown by day, who made the result,
  a share map, a monthly return heatmap, an income calendar, composition as one bar, income by
  classification, and where each category stands against its target. *Income by classification*
  shows what each category of a tree paid over the period, which is not the same as what it is
  worth: see the income topic.
- *Lists* — upcoming contributions, expected dividends, a watchlist, level crossings, dates reached, instrument events,
  largest positions, top performers, cash accounts, closed trades, risk figures, target values.
  *Expected dividends* is a forecast, never income: see the income topic for how it is made and
  why an amount may be gross.
- *Text* — a section heading, and the portfolio summary written by the assistant.

**The portfolio summary tile is not a chat.** It reads a fixed set of figures named by the app,
asks the model once, and keeps the text it got. It never generates on its own: the first generation
is the user pressing the button after being shown what the tile will read. It can be given an
instruction of its own ("what moved the portfolio this period", or a different question entirely)
and an interval at which it rewrites itself, which is off unless the user turns it on. When the
portfolio changes underneath it, the tile says its text is out of date rather than silently
spending money to redo it.

**Answer length, tokens** caps how long the summary is. The model is told the budget and plans the
text to fit it, so it leaves things out rather than stopping mid-sentence; a token is roughly three
quarters of an English word and less of a Russian one. Left empty, the model decides the length.
If the answer still runs past the ceiling, the tile reports that it was cut off and keeps its
previous text.

The tile's settings also choose who writes it: **Provider** and **Model**. Left empty, it uses the
provider and model a new chat starts on: the ones last picked in a chat, or that provider's
smallest model if none was picked. Every built-in provider is
listed, plus the user's own server once it is configured in Settings; one without a saved key is
shown as "no key saved" and cannot be picked. If the model refuses to answer or runs out of room mid-answer, the tile
says so and keeps its previous text.

**The portfolio and benchmark chart can compare against several instruments at once** — up to
five, each its own coloured dashed line, added one per **Benchmark** field in the tile's settings;
emptying a field removes that line. Every line is growth of one invested unit from the start of the
period, so the lines are comparable with each other and with the portfolio. An instrument younger
than the period starts its line where its prices start, and one with no prices at all in the period
is named above the chart instead of drawn.

The same tile's **Draw inflation** switch adds one more line: the cost of money in the portfolio's
price-index region, which is set under Portfolio in Settings. It is a step rather than a smooth
curve, because a price index is published once a month, and it stops at the last published month
instead of running flat to the right edge. Where the portfolio's line sits above it, the portfolio
gained purchasing power; below it, it lost some despite any gain in money terms.

Charts fill the tile they are in, so a tile dragged taller is drawn into rather than padded out.
