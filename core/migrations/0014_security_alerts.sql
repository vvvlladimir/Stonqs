-- Limit prices, dates to act on, and dated events of an instrument. See ADR-0034.

-- A rule about one instrument. Whether it has fired is derived from quotes and the calendar;
-- only what cannot be derived is stored: which firing the user dismissed and which one the
-- OS was already told about, each as the date that firing began.
CREATE TABLE security_alerts (
    id              TEXT PRIMARY KEY,
    security_id     TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,
    -- PRICE_ABOVE / PRICE_BELOW: the limit and the currency it is written in.
    price           TEXT,
    currency        TEXT,
    -- DATE_REACHED: the day the rule fires.
    date            TEXT,
    note            TEXT,
    created_on      TEXT NOT NULL,
    acknowledged_for TEXT,
    notified_for    TEXT
);
CREATE INDEX idx_security_alerts_security ON security_alerts (security_id);

-- A dated fact about an instrument: the user's note, or a dividend or split its quote provider
-- reported. `source` is NULL for the user's own events.
CREATE TABLE security_events (
    id          TEXT PRIMARY KEY,
    security_id TEXT NOT NULL REFERENCES securities (id) ON DELETE CASCADE,
    date        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    amount      TEXT,
    currency    TEXT,
    ratio_from  TEXT,
    ratio_to    TEXT,
    note        TEXT,
    source      TEXT
);
CREATE INDEX idx_security_events_security ON security_events (security_id, date);
-- A provider reports one dividend or split per day; a re-fetch updates it instead of adding one.
CREATE UNIQUE INDEX idx_security_events_provider
    ON security_events (security_id, kind, date) WHERE source IS NOT NULL;

-- What was asked of the provider for events, kept apart from `quote_coverage` so a database
-- whose quotes predate this table still backfills its dividends and splits once.
CREATE TABLE event_coverage (
    security_id TEXT PRIMARY KEY REFERENCES securities (id) ON DELETE CASCADE,
    from_date   TEXT NOT NULL,
    to_date     TEXT NOT NULL
);
