-- Classifiable cash balances and per-taxonomy exclusions.

-- Cash uses a separate table because it has no security_id and may hold multiple currencies.
CREATE TABLE cash_classifications (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    currency   TEXT NOT NULL,
    node_id    TEXT NOT NULL REFERENCES taxonomy_nodes (id) ON DELETE CASCADE,
    weight     TEXT NOT NULL,
    PRIMARY KEY (account_id, currency, node_id)
);
CREATE INDEX idx_cash_classifications_node ON cash_classifications (node_id);

-- Exclusions are scoped to a taxonomy and preserve existing classifications.
-- subject_id may identify either a security or a cash balance, so no single FK applies.
CREATE TABLE taxonomy_exclusions (
    taxonomy_id TEXT NOT NULL REFERENCES taxonomies (id) ON DELETE CASCADE,
    subject_id  TEXT NOT NULL,
    PRIMARY KEY (taxonomy_id, subject_id)
);
