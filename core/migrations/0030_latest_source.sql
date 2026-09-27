-- Which other source, if any, is asked for the days the instrument's own source has not published
-- yet: history from the own source, the latest close from this one. At most one per instrument.
-- Every existing row is a fallback only. See ADR-0079.
ALTER TABLE security_symbols ADD COLUMN latest INTEGER NOT NULL DEFAULT 0;
CREATE UNIQUE INDEX security_symbols_one_latest ON security_symbols (security_id) WHERE latest = 1;
