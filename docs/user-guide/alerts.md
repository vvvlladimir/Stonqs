# Alerts

Rules the user sets on instruments, the log of what actually fired, and dated events of the
instruments themselves. Alerts are about instruments, so the account picker does not apply here.

**A price alert is a level, not a direction of hope.** The rule is "this instrument, this level,
in this currency", plus which crossings to log: upwards, downwards, or both. Every change of side
between two closes is a crossing, and it is written down with the level and the close as they were
at that moment — so the log stays true even after the level is edited.

A rule also starts watching from the day it is created. A level crossed last month is not reported
as news by a rule written today.

**A date alert** simply says "this day has arrived" for an instrument — a review date, a maturity,
a reminder to look again.

The list shows each rule with where the price stands against its level and how far it has to move
to reach it. A rule whose instrument has no quote yet, or whose level is in a currency with no
exchange rate on file, says it is waiting for data — it never reports a made-up distance.

**Crossings are checked when quotes are refreshed**, not continuously. The app is not a trading
terminal: it reads closes. A level crossed and recovered inside one day between two closes is not
seen at all.

**The log** opens from the rules panel and lists crossings newest first. Unread crossings put a dot
on the navigation; opening the log clears the dot, while the lines keep their highlight for as long
as the log is open so nothing disappears while it is being read. A crossing also raises a system
notification once, the first time it is seen.

**Instrument events** are the other half of the screen: dividends and splits the quote provider
reported, and notes the user wrote themselves. Provider events are facts, so only a note can be
added to them; the date and figures stay the provider's. A reported split is *offered* as a
corporate action — it never adjusts holdings by itself, because that is a change to the ledger and
only the user makes those.
