-- What a plugin keeps in this profile: one opaque document per plugin, never read by the app.
-- In the database rather than beside it, so an encrypted profile encrypts it too. Removing the
-- plugin leaves its row: uninstalling is not deleting the user's data. See ADR-0084.
CREATE TABLE plugin_state (
    plugin     TEXT PRIMARY KEY,
    state      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
