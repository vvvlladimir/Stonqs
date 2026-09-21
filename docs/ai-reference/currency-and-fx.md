# Currency and FX

The portfolio has one base currency, and every total is expressed in it. Three currencies can be in
play for a single position and conflating them produces numbers that look plausible and are wrong:

- the **quote currency** — what the listing trades in,
- the **cost currency** — what the user actually paid from, fixed on the trade,
- the **base currency** — what the report is in.

Exchange rates are read as of a date, or the last known before it, like prices. A rate that is
missing is reported as missing, never assumed to be 1.

Rates come from the European Central Bank's daily reference rates wherever it publishes the
currency — about thirty majors. A currency it does not publish (the Russian rouble since March
2022, the Argentine peso, most emerging-market currencies) is taken from the market's own daily
close instead. If the central bank cannot be reached, a mirror of the same reference rates is
asked, and only then the market close. A fallback only fills days that have no rate yet; it never
replaces one already stored. The sources differ by a fraction of a percent at most. A currency neither source knows stays missing and every figure depending on it shows as
unavailable.

The rate used on a trade is the rate of that trade, fixed when it happened. Later moves in the
exchange rate do not rewrite the cost of a purchase made two years ago — they change what it is
worth today, which is a different number.

Some venues quote in a minor unit: London in pence, Johannesburg in cents, Tel Aviv in agorot. The
app folds those into the major currency, so a price of 5,234 pence is 52.34 GBP. A figure a hundred
times too large is almost always this.

For a portfolio in one base currency, part of every return is currency movement rather than the
instrument. When the question is "why did this fall when the market rose", the exchange rate is a
real candidate and worth naming.
