-- AI assistant chat history. `content` is the host's own JSON (role/blocks) — this table stores
-- it as an opaque string and never interprets it; the AI provider layer lives in sq-app, not here.
-- See .claude/rules/ui-boundary.md.

CREATE TABLE ai_chats (
    id         TEXT PRIMARY KEY,
    title      TEXT NOT NULL,
    provider   TEXT NOT NULL,
    model      TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Read back in insertion order (rowid), never by created_at: two turns in the same exchange can
-- share a timestamp, but never a rowid.
CREATE TABLE ai_messages (
    id         TEXT PRIMARY KEY,
    chat_id    TEXT NOT NULL REFERENCES ai_chats (id) ON DELETE CASCADE,
    role       TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_ai_messages_chat ON ai_messages (chat_id);
