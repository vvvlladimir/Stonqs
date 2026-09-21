-- What each request to a model cost, as the provider counted it.
--
-- One row per *request*, not per turn: a turn that calls tools is several requests, and each is
-- billed on its own. `chat_id` is nullable and set to NULL rather than deleted with the chat —
-- the dashboard brief has no chat at all, and spending already paid for is not rewritten by
-- deleting the conversation it happened in.
--
-- A figure a provider does not report is stored as 0: the neutral shape is the same for every
-- adapter, and "not reported" and "none" are not worth a second column here.
CREATE TABLE ai_usage (
    id               TEXT PRIMARY KEY,
    chat_id          TEXT REFERENCES ai_chats(id) ON DELETE SET NULL,
    provider         TEXT NOT NULL,
    model            TEXT NOT NULL,
    input_tokens     INTEGER NOT NULL DEFAULT 0,
    cached_tokens    INTEGER NOT NULL DEFAULT 0,
    output_tokens    INTEGER NOT NULL DEFAULT 0,
    reasoning_tokens INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL
);

CREATE INDEX ai_usage_chat ON ai_usage (chat_id);
