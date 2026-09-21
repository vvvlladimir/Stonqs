-- Classification trees and security assignments.

CREATE TABLE taxonomies (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    -- Tree role: ASSET_CLASS, REGION, SECTOR, or CUSTOM.
    kind TEXT NOT NULL
);

CREATE TABLE taxonomy_nodes (
    id           TEXT PRIMARY KEY,
    taxonomy_id  TEXT NOT NULL REFERENCES taxonomies (id) ON DELETE CASCADE,
    -- NULL is a root; self-reference supports arbitrary tree depth.
    parent_id    TEXT REFERENCES taxonomy_nodes (id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    -- Explicit sibling order keeps the user-defined presentation order.
    rank         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_taxonomy_nodes_tree ON taxonomy_nodes (taxonomy_id, parent_id, rank);

-- A security may split across weighted nodes, which is required for broad funds.
CREATE TABLE security_classifications (
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    node_id     TEXT NOT NULL REFERENCES taxonomy_nodes (id) ON DELETE CASCADE,
    weight      TEXT NOT NULL,
    PRIMARY KEY (security_id, node_id)
);
CREATE INDEX idx_security_classifications_node ON security_classifications (node_id);
