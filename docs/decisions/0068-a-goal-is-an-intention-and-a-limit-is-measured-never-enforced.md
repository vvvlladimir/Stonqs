# 68: A goal is an intention, and a contribution limit is measured, never enforced

- Status: Accepted (withdrawal netting superseded by ADR-0071)

## Context

Two things every other tracker has and this one does not: a **savings goal** ("have 20 000 by June
2027, these accounts count towards it") and a **contribution limit** ("this account takes at most
20 000 a year") — the ISA, 401(k) and ИИС ceiling.

Neither is a new measurement. A goal compares what a named set of accounts holds against an amount
the user typed; a limit compares what was paid into one account this limit year against a ceiling
the user typed. What has to be decided is where they live, what counts, and how much of the
world's tax code ships in the binary.

FIRE (ADR-0059) is the neighbour and is deliberately not the same question: it asks "what would it
take to stop working", from assumptions, with no amount and no date of its own.

## Decision

**A goal carries its own accounts and ignores the picker.** Like an investment plan (ADR-0033) it
is an intention about the portfolio, so `goals_list` and `goal_progress` read `portfolio()`.
Narrowing the lens must not make a goal look unfunded. A goal with no accounts named is about the
whole portfolio; one that names accounts is about exactly those, whatever the picker says — the
goal is its own lens. The reading *date* is the app's own, like every other reading, because
"what is it worth" always has a day.

**A goal is saving, not forecasting.** Its expected return is the user's, defaults to zero and is
never read from the portfolio's own past — the same rule FIRE already follows and for the same
reason. With a target date, the answer is the monthly contribution that would close the gap by
then; without one, it is the date the stated monthly contribution would arrive. Both solve the
identical annuity `fire::months_to_reach` already solves, so the two tiles cannot disagree about
what compounding means.

**A contribution is money that entered the portfolio.** A deposit, and an inbound transfer whose
partner leg is *not* inside the portfolio — the same `paired_links` reading that keeps TWR honest
(ADR-0020). Moving money between two of the user's own accounts is not a contribution and never
eats a limit; counting it would make one transfer look like a fresh 20 000 of allowance spent.
Withdrawals are netted off within the same limit year, because that is how an allowance is
administered where it can be refilled, and a negative result reads as zero used, not as allowance
gained.

**No jurisdiction ships in the binary.** A limit is an account, an amount, a currency and the day
its year starts (`MM-DD`, because the UK's begins on 6 April). No table of ISA/401(k)/ИИС rules,
no country field, no yearly upgrade of the app when a ceiling changes. The rules differ per country
*and per year*, and a wrong ceiling stated confidently is worse than no ceiling at all; a shipped
table is also exactly the kind of data a plugin should carry.

**A limit is measured, never enforced.** Nothing refuses a transaction, warns on entry, or blocks
an import. The app reports what was paid in against the ceiling; what to do about it is the user's
business, and an app that refused a deposit it had merely mis-classified would be worse than one
that reports a number the user can check.

## Alternatives

- **Scoping goals through the account picker.** Reads as cheaper than storing the accounts, but
  makes the same goal answer differently depending on where the user happens to be looking.
- **A country field with shipped ceilings.** Instantly wrong for every year the build predates,
  and a per-country table in the core is the definition of what does not belong there.
- **Enforcing the limit at entry.** Turns a reporting feature into a gate on data entry; a
  misread transfer would then stop the user recording what really happened.
- **Reusing an investment plan as the goal.** A plan is what will be bought and when; a goal is an
  amount by a date. Overloading one on the other would make "pause the plan" mean "abandon the
  goal".

## Consequences

- A goal and a limit are both user data and both survive without any market data: they read a
  valuation and a ledger, and never the network.
- A goal named over accounts that are later deleted loses those members and keeps working over
  what is left; it is not silently re-pointed at the whole portfolio.
- Because a contribution is defined by the ledger, an unpaired transfer leg — one whose partner
  was never imported — counts as a contribution. That is the same reading every return figure on
  the screen already takes, so the two agree; the fix is to link the legs, not to special-case it
  here.
- Nothing in the app knows a tax year is a tax year. A user with two allowances and one account
  states two limits, and the app shows both.
