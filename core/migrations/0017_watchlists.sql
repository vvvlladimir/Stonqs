-- Named lists of instruments the user follows, held or not. See ADR-0035.

-- Lists read in the order they were created (rowid); an upsert keeps the rowid.
CREATE TABLE watchlists (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

-- Deleting an instrument takes it off every list; nothing else refers to a list.
CREATE TABLE watchlist_items (
    watchlist_id TEXT NOT NULL REFERENCES watchlists (id) ON DELETE CASCADE,
    security_id  TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    position     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (watchlist_id, security_id)
);
CREATE INDEX idx_watchlist_items_security ON watchlist_items (security_id);
