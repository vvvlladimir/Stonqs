# 1: ECB as the FX provider


- Status: Accepted

## Context

Need a source of historical FX rates for portfolio valuation. It must be free,
require no API key, and cover history back to at least the earliest supported
transactions.

## Decision

Use the European Central Bank's SDMX reference-rate feed (`fx/ecb.rs`).

It is free, keyless, has history back to 1999, has a stable CSV format, and —
most importantly for accounting — is the *official* rate that European
brokers and tax authorities cite. Matching the broker's own statement matters
more here than quote freshness.

### Everything is EUR-denominated at the source

ECB only publishes `EUR/X` pairs ("how much X for one euro"), so the provider
handles three cases:

- `EUR -> X`: direct series.
- `X -> EUR`: same series, inverted (`1 / rate`).
- `X -> Y`: two series, cross-rate `rate(EUR→Y) / rate(EUR→X)`, computed only
  on dates where both sides are known.

Synthesizing a cross-rate is acceptable here — unlike at the storage-lookup
layer, where it's forbidden — because both halves come from one source, one
date, one methodology. There's no risk of splicing data from different
origins.

### CSV parsing shortcut

The response has 32 columns, some of which are quoted text containing
commas. A full CSV parser isn't needed: the fields we want, `TIME_PERIOD`
(index 6) and `OBS_VALUE` (index 7), sit *before* the first quoted field, so
a naive `split(',')` reaches them correctly. This is a deliberate
simplification, not an oversight.
