# 71: A withdrawal restores allowance only when the limit says so

- Status: Accepted

## Context

ADR-0068 netted withdrawals off a contribution limit inside its year. On a real account that
is also spent from — a cash account with dozens of card payments a month — withdrawals exceed
deposits every year, so the limit read 0 used whatever was paid in, and a new deposit changed
nothing on screen. It is also wrong for most allowances: an ordinary UK ISA or a Russian ИИС stays
spent once a deposit is made, and taking the money out does not give it back. Only a "flexible"
ISA restores what was withdrawn in the same year.

## Decision

A limit carries `withdrawals_restore` (`contribution_limits.withdrawals_restore`, migration 0029),
off by default and off for every existing limit. Off, `used` counts only what entered the
portfolio through the account — deposits and incoming unpaired transfers — and ignores what left.
On, withdrawals and outgoing unpaired transfers are netted off inside the limit year, exactly as
ADR-0068 did, and a negative year still reads as zero. The app still ships no jurisdiction: the
switch is the user's, like the amount and the day the year opens.

## Alternatives

- Keep netting and let the user fix it by choosing another account: the deposits land where the
  spending happens, so there is no other account to choose.
- Stop netting with no switch: simpler, but misreports a flexible ISA, where a withdrawal really
  does free the allowance again.

## Consequences

Existing limits change meaning on upgrade: a limit that showed 0 because of spending now shows the
year's deposits. The wire gains `withdrawals_restore` on `LimitUsage` and `LimitInput`; an input
without it reads as off.
