# 7: Resolve listings through MIC and verified provider symbols


- Status: Accepted

## Context

An ISIN can have several venue-specific tickers, and provider symbols use
different venue dialects. A search result may contain a name and currency but
still have no historical candles.

## Decision

Store ISO 10383 MIC values as the venue identifier. Listing directories return
ISIN/MIC/ticker pairs; quote sources convert those pairs into provider symbols.
OpenFIGI is queried per supported MIC so the request itself supplies the MIC
that its response omits.

Treat directory results and search results as unverified. Probe candidate
profiles and prefer a candidate with history, using the portfolio's base
currency as a soft tie-breaker. Reject provider symbols that are still ISINs.

## Consequences

The model preserves the user's venue choice without storing provider-specific
exchange codes. Resolution avoids unusable placeholder listings and keeps
quote currency attached to the verified price series.
