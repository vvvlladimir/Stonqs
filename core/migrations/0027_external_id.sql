-- The broker's own identifier for a row. A restated statement prints the same id with new
-- values, so identity stops being "the same numbers" and becomes "the same operation".
ALTER TABLE transactions ADD COLUMN external_id TEXT;

CREATE INDEX IF NOT EXISTS idx_transactions_external_id
    ON transactions (external_id) WHERE external_id IS NOT NULL;
