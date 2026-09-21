# Inflation and real return

A nominal return says how much more money you have. A **real** return says how much more you can
buy with it. The app reports both, and they answer different questions: a portfolio that gained
9% in a year prices rose 4% did not gain 9% in purchasing power.

## Real return is a division, not a subtraction

The app computes real return as *(1 + nominal) divided by the inflation factor, minus one* — not
nominal minus inflation. The inflation factor is what one unit of money at the start of the period
costs at its end: 1.04 means prices are 4% higher.

Over the example above, subtraction says 5.00% and the app says 4.81%. The gap looks like rounding
over one quiet year and is not: money earned is spent at the later price level, so the two
compound. Over a decade, or in an economy with double-digit inflation, the difference is large.

Real figures are produced for the time-weighted return and for the money-weighted (internal rate
of) return. The money-weighted one is deflated flow by flow: a contribution made three years ago
was made in cheaper money and is restated at *its own* date's price level, not at the period's.
Deflating the final answer instead would quietly credit every past payment with today's
purchasing power.

## It is off until you name a region

Real return is reported only once the portfolio names a **region** — the place whose prices you
actually pay. This is deliberately not derived from the base currency. Reporting in dollars says
nothing about which shops you walk into, and several countries use the same currency. Until a
region is set, the app shows no real figures at all, which is different from showing zero
inflation.

The region is a property of the portfolio, so it stays with it and is not part of the data scope:
narrowing to one account does not change which country's prices are used. The lens still applies
to the *return* being deflated.

## The window stops where the data stops

A country's price index for a month is published weeks after that month ends. A period running to
today therefore cannot be deflated all the way to today, and the app does not pretend otherwise:
it deflates through the **last published month** and reports the window it actually measured,
which may end earlier than the period you asked for. The nominal return beside it still covers the
full period. Where the two windows differ, the real figure understates slightly — the unpublished
tail is left out rather than guessed.

The index is a monthly fact, so the inflation line steps: one level holds until the next is
published. Nothing is interpolated into daily inflation, because no one measures it daily.

## Where the numbers come from

Two free public sources cover the world between them: a European harmonised index for Europe, the
euro area and a few others, and an international one for everywhere else. A region's whole series
comes from one of them, never a mix. That is not caution about quality — publishers set their
index to 100 in different years, so stitching two series together would produce a jump that
measures the change of base year rather than any change in prices.

The level itself has no meaning on its own; only the ratio between two dates does. Two portfolios
reporting against different regions cannot have their inflation factors compared directly either
— each is measured against its own country's basket.

If a region is set but its index has not been fetched yet, the real figures are reported as
missing, exactly like an absent price, rather than as no inflation.
