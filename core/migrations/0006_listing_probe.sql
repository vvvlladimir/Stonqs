-- Add listing probe fields in a new migration; applied migrations must never be edited.
-- Both fields are populated by the same provider request as currency and has_history.

-- Instrument name returned by the quote provider.
ALTER TABLE listings ADD COLUMN name TEXT;

-- Reference-only price for listing selection; calculations always use `quotes`.
-- Stored as TEXT to preserve decimal precision.
ALTER TABLE listings ADD COLUMN last_close TEXT;
