-- Portfolio targets and security quantity steps.

-- Minimum tradable quantity; NULL means no broker-specific step is configured.
ALTER TABLE securities ADD COLUMN quantity_step TEXT;

-- Targets are separate from taxonomy nodes because one tree can serve many portfolios.
CREATE TABLE targets (
    id           TEXT PRIMARY KEY,
    portfolio_id TEXT NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    taxonomy_id  TEXT NOT NULL REFERENCES taxonomies (id) ON DELETE CASCADE,
    name         TEXT NOT NULL
);
CREATE INDEX idx_targets_portfolio ON targets (portfolio_id);

CREATE TABLE target_weights (
    target_id TEXT NOT NULL REFERENCES targets (id) ON DELETE CASCADE,
    node_id   TEXT NOT NULL REFERENCES taxonomy_nodes (id) ON DELETE CASCADE,
    -- Weight in [0, 1]; the sum may be below one to leave assets outside the target.
    weight    TEXT NOT NULL,
    PRIMARY KEY (target_id, node_id)
);
