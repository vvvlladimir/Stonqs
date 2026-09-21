-- Facts about an instrument that no provider supplies: a note, the German WKN,
-- and attributes the user invents (TER, country of risk, replication method).

ALTER TABLE securities ADD COLUMN note TEXT;
ALTER TABLE securities ADD COLUMN wkn TEXT;

-- One row per attribute the user defined. The name is user data and is never translated;
-- the kind decides how a value is parsed, the unit only how it is shown.
CREATE TABLE security_attribute_defs (
    id       TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    kind     TEXT NOT NULL,
    unit     TEXT,
    position INTEGER NOT NULL DEFAULT 0
);

-- The value is TEXT whatever the kind, like every other number and date in this database.
CREATE TABLE security_attributes (
    security_id  TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    attribute_id TEXT NOT NULL REFERENCES security_attribute_defs (id) ON DELETE CASCADE,
    value        TEXT NOT NULL,
    PRIMARY KEY (security_id, attribute_id)
);
CREATE INDEX idx_security_attributes_attribute ON security_attributes (attribute_id);
