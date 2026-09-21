-- Cached listings map an ISIN to venue-specific symbols and currencies.
-- They avoid arbitrary search results and repeated OpenFIGI requests.
CREATE TABLE listings (
    isin        TEXT NOT NULL,
    -- ISO 10383 market identifier (MIC), independent of provider-specific codes.
    mic         TEXT NOT NULL,
    -- Venue-local ticker; the same instrument may use different tickers by country.
    ticker      TEXT NOT NULL,
    -- Provider symbol, or NULL when no venue mapping is known.
    symbol      TEXT,
    -- Cached probe result; NULL means the listing has not been checked.
    -- Name and last_close were added later in migration 0006.
    currency    TEXT,
    has_history INTEGER,
    source      TEXT NOT NULL,
    fetched_at  TEXT NOT NULL,
    PRIMARY KEY (isin, mic, ticker)
);
CREATE INDEX idx_listings_isin ON listings (isin);
