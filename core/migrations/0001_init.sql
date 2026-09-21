-- Initial schema. Decimal values use TEXT because SQLite REAL cannot preserve them exactly.
-- Dates use ISO text, whose lexical order matches chronological order; booleans use INTEGER 0/1.

CREATE TABLE accounts (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    currency   TEXT NOT NULL,
    kind       TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    opened_at  TEXT
);

CREATE TABLE securities (
    id          TEXT PRIMARY KEY,
    symbol      TEXT NOT NULL,
    isin        TEXT,
    name        TEXT NOT NULL,
    currency    TEXT NOT NULL,
    kind        TEXT NOT NULL,
    data_source TEXT,
    data_symbol TEXT
);
-- One symbol identifies one security; duplicates would silently corrupt reports.
CREATE UNIQUE INDEX idx_securities_symbol ON securities (symbol);

CREATE TABLE portfolios (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    base_currency TEXT NOT NULL
);

-- Many-to-many relation: an account may belong to multiple portfolios.
CREATE TABLE portfolio_accounts (
    portfolio_id TEXT NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    account_id   TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    PRIMARY KEY (portfolio_id, account_id)
);

CREATE TABLE transactions (
    id              TEXT PRIMARY KEY,
    account_id      TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    security_id     TEXT REFERENCES securities (id) ON DELETE RESTRICT,
    kind            TEXT NOT NULL,
    date            TEXT NOT NULL,
    quantity        TEXT NOT NULL,
    price           TEXT NOT NULL,
    amount          TEXT NOT NULL,
    fees            TEXT NOT NULL,
    taxes           TEXT NOT NULL,
    currency        TEXT NOT NULL,
    fx_rate_to_base TEXT,
    note            TEXT
);
-- Primary read pattern: all transactions for an account ordered by date.
CREATE INDEX idx_transactions_account_date ON transactions (account_id, date);
CREATE INDEX idx_transactions_security_date ON transactions (security_id, date);

CREATE TABLE quotes (
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    close       TEXT NOT NULL,
    currency    TEXT NOT NULL,
    source      TEXT NOT NULL,
    -- One quote per security and date; refreshes upsert instead of duplicating rows.
    PRIMARY KEY (security_id, date)
);

-- Tracks requested ranges separately from returned quotes, including non-trading days.
CREATE TABLE quote_coverage (
    security_id TEXT PRIMARY KEY REFERENCES securities (id) ON DELETE CASCADE,
    from_date   TEXT NOT NULL,
    to_date     TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE fx_rates (
    base  TEXT NOT NULL,
    quote TEXT NOT NULL,
    date  TEXT NOT NULL,
    rate  TEXT NOT NULL,
    PRIMARY KEY (base, quote, date)
);
