-- Consumer-price index levels, one row per region and month.
-- The month is stored as its first day so lexicographic order stays chronological.
-- `source` is recorded because index bases differ between publishers (2015=100 vs 2010=100):
-- a ratio is only meaningful inside one source's series.
CREATE TABLE price_index (
    region TEXT NOT NULL,
    month  TEXT NOT NULL,
    value  TEXT NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (region, month)
);

-- What was already asked for, mirroring quote_coverage. One row per region: a region's
-- series comes from exactly one source at a time.
CREATE TABLE index_coverage (
    region     TEXT PRIMARY KEY,
    from_date  TEXT NOT NULL,
    to_date    TEXT NOT NULL,
    source     TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Where the portfolio's owner spends. NULL means inflation is not reported at all, which is
-- the state every existing portfolio starts in.
ALTER TABLE portfolios ADD COLUMN inflation_region TEXT;
