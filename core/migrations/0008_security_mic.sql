-- Preserve the selected venue on the security; a local ticker cannot identify it.
-- Store the ISO 10383 MIC, with NULL meaning no listing was selected.
ALTER TABLE securities ADD COLUMN mic TEXT;
