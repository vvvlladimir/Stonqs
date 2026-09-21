# 60: Inflation is a region the owner names, and a real return divides

- Status: Accepted

## Context

A return of 9% in a year prices rose 4% is not a 9% gain in what the money buys. Portfolio
Performance answers this with a consumer-price series the user picks; we had no price index at
all, and no way to say what a figure is worth.

Three things had to be decided. Where the region comes from: a portfolio reports in a base
currency, but reporting in USD says nothing about which shop the owner walks into — a German
holding an American portfolio spends euros. Which source: the app is an international tracker, so
a European-only index is not enough, and nothing free covers the whole world on its own. And how
two sources combine, which is where the trap is: publishers rebase their indices on different
years, so gluing two series together produces a ratio that measures the rebasing.

## Decision

**The region is a property of the portfolio**, beside `base_currency` and derived from neither it
nor the locale. `Portfolio::inflation_region` is an ISO 3166-1 alpha-2 code plus the two
aggregates Eurostat publishes (`EA`, `EU`); `None` means real returns are not reported, which is
what every existing portfolio starts as.

**Two sources, chained like FX rates.** Eurostat's harmonised index first for the 40 geographies
it covers — it carries the euro-area aggregate the IMF does not and publishes sooner — then the
IMF's consumer price index for the other 191 economies. Both are free, keyless and monthly. Each
is a row in `sources::CATALOG` with a new `Capability::PriceIndex`, so nothing names them by
string, and `sources::index_service()` assembles the chain the way `fx_service()` does.

**A fallback replaces a region's series rather than filling into it.** This is the one place the
FX rule is deliberately inverted: `save_fx_rates_from(.., false)` fills gaps because two rate
sources quote the same number, while two index sources quote the same prices against different
base years. One region, one source, one base — or the ratio of two months is a fiction.

**A real return divides**: `(1 + nominal) / factor - 1`, never `nominal - inflation`. The factor is
`I(to) / I(from)` inside one series. The money-weighted return deflates **each flow at its own
date** before solving, because a contribution made three years ago was made in cheaper money.

**The window is reported, not assumed.** A month's index is published weeks after the month ends,
so a period running to today is deflated only through the last published month, and `RealReturn`
carries the `from`/`to` it actually measured — the same rule `BenchmarkComparison` follows for a
benchmark younger than the period. The index steps: a monthly level holds until the next is
published, and nothing interpolates a daily inflation rate nobody measured.

## Alternatives

- **Derive the region from the base currency.** Free, and wrong for every reader who does not live
  where their reporting currency is legal tender. It also has no answer for USD, which several
  economies use.
- **Store the published year-on-year rate instead of the level.** Smaller, but then the app holds
  two truths: the rate the source derived and the rate our own dates imply. Levels are the fact;
  every rate is arithmetic over them.
- **One source only.** Eurostat alone leaves out Japan, Russia, India, Brazil and everything else
  outside Europe. The IMF alone has no euro-area aggregate and publishes later.
- **Let a fallback fill gaps, like FX.** It reads as consistency and produces a series whose ratio
  measures a change of base year. Rejected outright.
- **Subtract inflation from the return.** The usual shorthand. It is close enough in a quiet year
  in a rich economy and wrong everywhere else, which is exactly the population the app is for.

## Consequences

- A new migration (`0025_price_index.sql`): `price_index`, `index_coverage`, and
  `portfolios.inflation_region`.
- `calc` gains an `IndexLookup` seam beside `PriceLookup` and `RateLookup`, so real returns are
  testable without a network, and `inflation_series` returns the same `GrowthSeries` a benchmark
  does — the chart draws one more line rather than growing a second concept.
- The refresh job asks for the index once a month rather than every run.
- Region codes cross IPC, never country names: the frontend has the language and can write them.
- A region whose index has not been fetched yet is `MissingMarketData`, like any other absent
  series. A portfolio with no region at all is `None` — a setting, not a gap.
- **A statistical office retires a dataset without retiring its endpoint.** Eurostat's ECOICOP
  ver. 1 table kept answering 200 for months after it stopped being updated, and a chain that
  falls through only on failure cannot see that: the answer was well-formed, merely old. The
  dataset version is therefore named at the call site, and one network test per source asks for a
  recent month rather than an old one, so a silent retirement fails a test instead of freezing a
  user's chart. The chain rule itself is unchanged — inventing a staleness threshold would be
  guessing at a publication calendar that differs per country.
