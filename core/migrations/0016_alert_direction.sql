-- A price trigger names the direction it fires in again. See ADR-0034.
-- Rules written while there was no choice logged every crossing, so they keep doing that.
ALTER TABLE security_alerts ADD COLUMN direction TEXT NOT NULL DEFAULT 'BOTH';
