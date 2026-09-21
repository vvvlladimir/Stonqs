-- Linked transactions, corporate actions, and cost-basis method.

-- Both sides of a transfer or currency exchange share one link_id.
ALTER TABLE transactions ADD COLUMN link_id TEXT;
CREATE INDEX idx_transactions_link ON transactions (link_id);

-- Corporate actions share one event shape so future mergers and spin-offs can reuse it.
CREATE TABLE corporate_actions (
    id          TEXT PRIMARY KEY,
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    -- A 2:1 split is stored as from=1, to=2; integer parts keep reverse splits readable.
    ratio_from  TEXT NOT NULL,
    ratio_to    TEXT NOT NULL,
    note        TEXT
);
CREATE INDEX idx_corporate_actions_security_date ON corporate_actions (security_id, date);

-- Cost basis belongs to the portfolio, allowing the same trades to be viewed by FIFO or average cost.
ALTER TABLE portfolios ADD COLUMN cost_basis_method TEXT NOT NULL DEFAULT 'FIFO';
