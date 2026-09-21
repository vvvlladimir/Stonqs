# 20: An unlinked cash transfer is an external flow

- Status: Accepted

## Context

`TransferIn`/`TransferOut` model money moving *inside* the portfolio: the pair is created
by `Transaction::cash_transfer` / `currency_exchange`, both legs share a `link_id`, and
`calc::holdings` deliberately records no external flow for them — an internal move must not
break a TWR period or count as a contribution.

Real broker exports do not respect that shape. A statement prints an incoming bank transfer,
an outgoing one and every card payment with wording that the import dictionary maps to
`TRANSFER` (see `.claude/rules/import.md`: the keyword is deliberately the reversible side,
because the same wording also covers currency and crypto conversions). Those rows arrive
one-sided — there is no counterpart leg anywhere in the portfolio. Treated as internal, the
money silently appears and disappears: card spending reads as a loss in the value series,
every contribution vanishes from TWR and XIRR, and the deposits/withdrawals lane of the
value chart is empty for the whole portfolio while a narrowed scope shows bars (there,
`calc::scoped_transactions` already rewrites an out-of-scope leg into an external flow).

## Decision

A cash transfer leg without `link_id` has no counterpart in the portfolio, so the money
crossed the portfolio boundary: `calc::holdings` records it as an external flow, signed by
`cash_delta` — an incoming leg like a deposit, an outgoing one like a withdrawal. A linked
pair stays internal, as before.

So that the one genuinely internal case keeps its `link_id`, `import::build_preview` pairs
the legs it can recognise: same date, same source wording, opposite direction — the "one
wording on two rows" shape a conversion, a stake or a wallet-to-wallet move has. The link id
is derived from those three, never random, because `build_preview` must stay reproducible.
A leg left without a partner stays unlinked and is therefore an external flow.

## Alternatives

- **Map the wordings to `Deposit`/`Withdrawal` in the import dictionary.** Fixes new imports
  only, leaves every already-imported portfolio wrong, and the same `TRANSFER` keyword has
  to keep covering conversions, which are not flows.
- **Pair unlinked legs inside `calc`.** `calc` sees no wording — only date, amount and kind —
  so it would have to guess from same-day mirrored amounts and would link a card payment to
  an unrelated incoming transfer.
- **Count both legs of an unpaired conversion as flows and accept the noise.** Numerically
  almost harmless (same-date legs net out in TWR and XIRR), but the chart would show a large
  fake deposit and withdrawal for every conversion.

## Consequences

- TWR, XIRR, contribution figures and the value chart's flow lane now see broker deposits
  and card spending for every scope, including the full portfolio.
- Existing databases are fixed without a migration: flows are derived, never stored.
- A user who records a one-sided transfer by hand states that money left or entered the
  portfolio. Recording a move between two own accounts requires both legs, which is what the
  paired constructors produce.
- `TransactionKind::is_external_flow` stays kind-only and therefore incomplete for transfers;
  the authority is `calc::holdings`.
