-- A price alert becomes one trigger level crossed either way, with a log of its crossings.
-- See ADR-0034.

-- Direction is no longer chosen by the user: a level above the price fires on the way up, one
-- below it on the way down, and every later crossing is logged as well.
UPDATE security_alerts SET kind = 'PRICE' WHERE kind IN ('PRICE_ABOVE', 'PRICE_BELOW');

ALTER TABLE security_alerts DROP COLUMN acknowledged_for;
ALTER TABLE security_alerts DROP COLUMN notified_for;

-- Which side of the level the last checked close was on, and that close's date. A crossing is a
-- change of side, so the check reads only quotes from `checked_through` on.
ALTER TABLE security_alerts ADD COLUMN side TEXT;
ALTER TABLE security_alerts ADD COLUMN checked_through TEXT;

-- The log. `level` and `currency` are copied from the rule, so editing the rule later does not
-- rewrite what was crossed. `seen` feeds the dot in the navigation, `notified` the OS notification.
CREATE TABLE alert_crossings (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    alert_id  TEXT NOT NULL REFERENCES security_alerts (id) ON DELETE CASCADE,
    date      TEXT NOT NULL,
    direction TEXT NOT NULL,
    level     TEXT,
    price     TEXT,
    currency  TEXT,
    seen      INTEGER NOT NULL DEFAULT 0,
    notified  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_alert_crossings_alert ON alert_crossings (alert_id, date);
