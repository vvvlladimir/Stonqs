-- Which source wrote a rate. Every row before this migration came from the ECB.
ALTER TABLE fx_rates ADD COLUMN source TEXT NOT NULL DEFAULT 'ecb';

-- An instrument's symbol at sources other than its own `data_source`. The own source keeps
-- reading `data_symbol`/`symbol`, so this table holds only the alternatives a chain may ask.
CREATE TABLE security_symbols (
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    source      TEXT NOT NULL,
    symbol      TEXT NOT NULL,
    PRIMARY KEY (security_id, source)
);
