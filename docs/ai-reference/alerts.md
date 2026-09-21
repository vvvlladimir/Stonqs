# Price alerts

An alert is a **level**, not a direction of its own: a close at or above the level is above it, and
every change of side between two consecutive closes is a crossing. The rule says which crossings to
log — up, down, or both.

Crossings are logged with the level and the close of that moment, so the log stays truthful even
after the level is edited later. Unseen crossings mark the alerts screen; opening the log marks them
seen.

An alert is about an instrument, so it is not affected by the account lens, and an instrument does
not have to be held for a level to be watched.

A level is given in the instrument's own quote currency. When that differs from the base currency,
the comparison is made at the rate of the quote's own day; if that rate is missing, the rule waits
for it rather than firing on a converted guess. An instrument with no quote at all has no alert
status — that is missing data, not "far from the level".

Alerts fire on closes. There is no intraday checking, and a level crossed and recrossed inside one
day leaves no trace.
