# Investment plans

A plan is a standing intention: this much money, on this schedule, into these instruments in these
proportions, from this account, with these flat costs. A plan with no instruments is a cash
contribution plan.

**A plan never writes anything by itself.** It offers the occurrences that have come due as drafts;
the user commits them, and only then do transactions exist. An occurrence and its transactions are
linked, so deleting the transactions offers that month again rather than leaving a gap.

"When was this plan last executed" is therefore derived from what was actually committed, not from a
date stored on the plan.

A plan is about the portfolio, not about the accounts currently in view, so narrowing the lens does
not hide it. A projection of future contributions is arithmetic on the plan, not a forecast of
returns, and should be described that way.

## Financial independence

The financial-independence tile answers one question: what the portfolio must be worth for a year
of spending to be withdrawn from it indefinitely, and when the current saving pace would get
there.

The target is the yearly spending divided by the withdrawal rate. A withdrawal rate of 0.04 — the
common rule of thumb — therefore makes the target twenty-five years of spending. The rate is not a
prediction and nothing verifies it; it is the assumption the user chose.

Only two of the inputs are measured: what the portfolio is worth today, and what the active plans
add up to in an average month. The monthly amount can be overridden, and a blank one means "what
the plans already say". The yearly spending, the withdrawal rate and the expected return are typed
by the user and nothing else in the app reads them.

The expected return is emphatically **not** this portfolio's measured return, and the calculation
never looks one up. It is also neither real nor nominal on its own: subtracting inflation before
typing it is what makes the answer read in today's money, and the tile cannot tell which was
given.

The horizon is the first month in which value plus contributions, compounded monthly at that
return, reaches the target — a whole month, because a target half-reached is not reached. It is
absent when the pace does not get there within a hundred years, including when it never can:
saving nothing into a portfolio that grows at nothing closes no gap. A portfolio already past its
target reads as reached today, and its progress reads above 100% rather than being capped.

This tile is not scoped by the account picker. Contributions belong to the portfolio rather than
to a lens, so reading one account's value against every account's savings would compare two
different things.
