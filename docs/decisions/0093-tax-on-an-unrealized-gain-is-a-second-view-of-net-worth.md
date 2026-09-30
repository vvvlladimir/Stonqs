# 93: Tax on an unrealized gain is a second view of net worth, and it covers only what has a cost

- Status: Accepted

## Context

Net worth (ADR-0092) counts what is owned at what it is worth. Part of that worth is a gain that
has never been taxed, and selling would hand some of it to a tax authority: two people with the
same net worth, one holding cost basis and one holding gain, do not have the same money. Kubera
shows a "net of tax" figure for this reason, and the question — "how much of this is actually
mine" — is asked constantly.

The app knows no jurisdiction and ships no rates (the same rule contribution limits follow,
ADR-0068). It does know one thing precisely: the portfolio's unrealized gain, per the cost basis
method the portfolio is kept under. About a flat it knows nothing of the kind — ADR-0092
deliberately gives an asset no purchase price, because a cost basis would imply a return, and a
return computed from two figures the owner typed is false precision.

So a tax view can be exact about the invested part and silent about the rest. What has to be
decided is whether to ship it at all on those terms, and where the rate comes from.

## Decision

**Ship it as a second view, named apart, and never as the total.** The reading is *net worth after
tax*, beside *net worth*, the way net worth itself sits beside portfolio value. No screen, tile or
tool replaces one with the other, and nothing else in the app reads it: it does not enter a
return, a goal, FIRE or an allocation.

**It covers only the portfolio's unrealized gain, and says what it left out.** The reading carries
the value of the things it could not speak about — everything owned outside the portfolio — so the
figure is never mistaken for "net worth after every tax I would ever pay". A flat's gain is
outside it by construction and stays outside until an asset has a purchase price, which is a
decision of its own and not this one.

**The rate is the owner's, stated as one number, and nothing is derived from a jurisdiction.**
One flat rate over the whole unrealized gain: no lot-level holding periods, no brackets, no
allowances, no loss carry-forward. A rate of zero is a legitimate answer and produces a tax of
zero, not an absent reading. A gain that is negative produces a tax of zero as well — a loss is
not a refund, and carrying it forward is a tax rule, which is exactly what the app does not model.
A rate outside 0–100% is refused rather than clamped.

**The rate lives where its owner acts on it: the screen.** It is an assumption a person types,
like FIRE's withdrawal rate (ADR-0059), so it is kept in the interface's own state and handed to
the reading as an argument. It is not a portfolio field and needs no migration, and the assistant
is not given it — a figure that depends on an assumption the model cannot see would be quoted as
fact.

## Alternatives

- **Per-asset rates, as Kubera asks for.** More faithful to how tax works and unusable without a
  cost basis per asset, which ADR-0092 refuses. It would also multiply the number of assumptions
  behind one headline figure.
- **A rate per jurisdiction, shipped.** Refused for the same reason contribution limits ship
  none: the rules differ per country *and* per year, and a wrong ceiling in a binary is worse than
  no ceiling.
- **Deducting the tax from net worth itself.** One total that quietly means "after an assumption
  somebody typed once" is the failure ADR-0092 exists to prevent.
- **Not shipping it.** Leaves the owner to divide in their head with a gain figure taken from
  another screen — the arithmetic is trivial, and doing it where both numbers already are is the
  whole value.

## Consequences

- One more figure whose scope has to be stated every time it is shown; the screen and the guide
  both name what it does not cover.
- The reading is exact where the app has a cost basis and silent elsewhere, which will read as
  incomplete to somebody with most of their worth in property. That is the honest shape of it.
- If an asset ever gains a purchase price, this reading is where that would show up, and this ADR
  is what would have to be superseded.
