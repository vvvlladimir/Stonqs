-- Things owned and owed that no market prices: a house, a car, a mortgage. See ADR-0092.

-- Not an account: nothing settles here and no transaction is recorded against it. The side of
-- `kind` says whether the amount is owned or owed, so every stored figure is positive.
-- `rate`/`monthly_payment`/`ends_on` are a debt's schedule and only ever look forward; what is
-- owed today is the latest row in `asset_values`.
CREATE TABLE assets (
    id              TEXT PRIMARY KEY,
    portfolio_id    TEXT NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    kind            TEXT NOT NULL,
    currency        TEXT NOT NULL,
    -- The asset a debt is secured by, or the debt against a thing owned. Shows a relationship
    -- and changes no figure; losing the other side leaves the asset, not a dangling id.
    secured_by      TEXT REFERENCES assets (id) ON DELETE SET NULL,
    rate            TEXT,
    monthly_payment TEXT,
    ends_on         TEXT,
    note            TEXT,
    -- Sold, repaid or written off: absent from net worth from this day on, history kept.
    closed_on       TEXT,
    CHECK ((rate IS NULL) = (monthly_payment IS NULL))
);
CREATE INDEX idx_assets_portfolio ON assets (portfolio_id);

-- One opinion per day, dated. The value on any day is the latest row on or before it — a step,
-- never an interpolation (ADR-0092), so re-valuing the same day replaces that day's figure.
CREATE TABLE asset_values (
    asset_id TEXT NOT NULL REFERENCES assets (id) ON DELETE CASCADE,
    date     TEXT NOT NULL,
    amount   TEXT NOT NULL,
    note     TEXT,
    PRIMARY KEY (asset_id, date)
);
