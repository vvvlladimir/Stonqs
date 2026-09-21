-- Consent the user gave for the rest of one chat: "this tool may read, in this conversation".
-- Only that scope is stored. "Allow once" is by definition not remembered, so a `scope` column
-- would describe a value the table never holds; a global grant does not exist at all.
--
-- There is deliberately no `ai_tool_calls` table: a call and its result are already two blocks of
-- the turn they belong to (`ai_messages.content`), and a second copy could disagree with the
-- history actually replayed to the model.

CREATE TABLE ai_grants (
    chat_id    TEXT NOT NULL REFERENCES ai_chats (id) ON DELETE CASCADE,
    tool       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (chat_id, tool)
);
