-- How hard the model is asked to think in this chat. Beside `tool_mode` and for the same reason:
-- it is a property of one conversation, switched from inside it, not an application setting.
--
-- `ai_chats.model` already exists and was only ever a record of what the chat was started with;
-- from here on it is also what the chat is *answered* with, so a model picked mid-chat sticks.

ALTER TABLE ai_chats ADD COLUMN effort TEXT NOT NULL DEFAULT 'MEDIUM';
