-- How a chat treats tool calls. `ASK` puts every not-yet-granted tool in front of the user;
-- `AUTO` runs read tools without asking, for the length of this one chat.
--
-- It lives on the chat rather than in settings because it is a property of one conversation, the
-- way Claude Code's permission mode belongs to a session: a chat started in `AUTO` to explore
-- must not quietly make the next one permissive too.

ALTER TABLE ai_chats ADD COLUMN tool_mode TEXT NOT NULL DEFAULT 'ASK';
