-- Requests spent per source and UTC day, so a keyed source's daily allowance survives a restart.
CREATE TABLE source_usage (
    source   TEXT NOT NULL,
    day      TEXT NOT NULL,
    requests INTEGER NOT NULL,
    PRIMARY KEY (source, day)
);
