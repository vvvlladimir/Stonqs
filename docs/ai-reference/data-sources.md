# Data sources

Prices and exchange rates come from outside sources, and the app can ask several of them. Which
ones it asks is set in Settings under Market data, in **Data sources**.

**Nothing is fetched until a source is chosen.** A new profile has every source off, keyless ones
included, and asks nobody. So missing prices on a fresh setup usually mean no source has been
chosen, not that a provider failed. Turning a source on in Settings is the choice itself — there
is nothing else to apply — and `Select all and continue` in the dialog a new profile is shown does
it for everything that answers without a key of its own.

**Two kinds of source are needed, not one.** Something that publishes prices, and something that
publishes exchange rates. Without the second, anything held in a currency other than the
portfolio's has no value at all rather than a stale one, so the app reports the gap until both are
on: the dialog refuses to apply such a set, and the data chip beside the search box shows an error
saying which half is missing.

**An instrument added while no source was on has no price source**, and nothing fetches its
prices even after a source is turned on later — a refresh only asks about instruments that name
one. Both the dialog and the Settings panel offer those instruments the first source turned on, in
one press; an instrument given a source of its own keeps it.

**Each instrument has its own price source** — the one chosen when it was added or identified.
That source is always asked first. Another source is asked only when the instrument's own one
fails (unreachable, throttling, a rejected key), and only if the instrument has a symbol at that
other source — set in the instrument's form under **Fallback symbols**, one per source, because
each source spells a ticker its own way. Its prices fill missing days and never replace ones already stored; they are also
checked first: a source whose closes are in another currency, or differ from the stored ones by
more than two percent, is ignored rather than mixed in. The next refresh asks the instrument's own
source again.

**Exchange rates** are asked in a fixed order: the European Central Bank for the roughly thirty
currencies it publishes, a mirror of the same rates if it is unreachable, then the market's daily
close for everything else. A currency no source knows stays missing, and every figure depending
on it shows as unavailable rather than as zero.

**Some sources need an API key** (Twelve Data, EODHD). Their free plans allow a fixed number of
requests a day; once that is spent the source is skipped until the next day (UTC) and the next
source is asked instead. Such a source shows **Needs a key** and is
not asked until one is saved with **Add key**. Keys are stored encrypted with the profile's
password, so a profile without a password must get one first; a locked profile asks no keyed
source. A key is never shown again. Any source can be switched with **Turn off** / **Turn on**;
turning off the source an instrument uses leaves that instrument without new prices.

Crypto can also be quoted by Kraken (crypto only, no key); it keeps about two
years of daily history.

**Your own sources**: any JSON or CSV price address can be added with **Add source** — its address
with placeholders such as the symbol and the date range, and where the dates and closes are in the
answer (a path like `$.values[*].close` for JSON, a column name for CSV). **Test** tries it on one
symbol before anything is saved. A source can answer **Prices of instruments** or **Exchange
rates**; a rate source takes the pair's two currencies in its address and is asked after every
built-in rate source. An instrument uses a price source like any other.

When a refresh fails, its message names the source that did not answer, and says so when that
source refused its API key.

A source that fails several times in a row is skipped for the rest of that refresh, so one broken
source does not slow down the whole update.
