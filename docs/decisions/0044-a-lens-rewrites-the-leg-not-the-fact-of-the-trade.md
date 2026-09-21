# 44: A lens rewrites the leg, not the fact of the trade

- Status: Accepted

## Context

`calc::scoped_transactions` rewrites a trade whose securities side is in scope and whose cash side is
not into a delivery (`as_delivery`). It has to: the cash that paid for the shares sits on an account
the lens cannot see, and keeping the row a `Buy` would drive that account's balance negative inside
the scope.

`trading_volume` excludes deliveries, and says why: "shares arriving from another broker cost no
commission and are not a decision this portfolio made."

Both statements are right on their own and contradict each other after the rewrite. `as_delivery`
changes `kind` and clears `link_id` but keeps `fees` — it must, because a buy commission is part of
the lot's cost basis, and clearing it would move the cost basis, the unrealised result and the return
of every position under that lens.

So a depot-only scope reported a fee rate of 1.21 % on 19.00 EUR of commissions beside a turnover of
0.0 % on 0.00 EUR traded: commissions with no trade to have caused them. The premise the delivery
exclusion rests on — that a delivery carries no commission — is exactly what the rewrite breaks.

## Decision

The rewrite records what it rewrote. `Transaction` gains

```rust
#[serde(skip)]
pub scoped_from: Option<TransactionKind>,
```

set only by `as_delivery`, and `trading_volume` reads `scoped_from.unwrap_or(kind)` when deciding
whether a row is a trade and which way it went. The gross is still read off the row as it stands:
`gross_in_transaction_currency` uses the same formula for `DeliveryInbound` as for `Buy`, and for
`DeliveryOutbound` as for `Sell`.

`as_external_cash` — the mirror rewrite, which turns the *cash* leg of an out-of-scope trade into a
deposit or a withdrawal — deliberately sets nothing. Seen from a deposit account the shares never
arrived and the trade happened somewhere else, so that lens trades nothing.

The field is never stored and never crosses IPC: it describes a view of a row, not the row. A row
read back from SQLite always has `None`, and so does every row the user or the assistant writes.

## Alternatives

- **Clear `fees` in `as_delivery`.** Restores the exclusion's premise and breaks cost basis, which
  is a far larger lie than a wrong turnover.
- **Count any delivery that carries a commission.** Works by accident and misses a commission-free
  purchase, which is the common case for several brokers.
- **A new `TransactionKind`.** This is not a new kind of operation; it is the same operation seen
  through a lens. A kind would reach storage, the import, the wire and every `match` in the code.
- **Report no turnover under a narrowed scope.** Refusing to answer is defensible, but the depot did
  buy, and the screen already shows what those purchases cost.

## Consequences

Turnover under a scope that sees a depot without its settlement account is no longer zero, and the
fee rate beside it now has trades to belong to. The whole-portfolio figure is unchanged — nothing is
rewritten there.

A genuine delivery still contributes nothing, under any scope. `Transaction` carries one field that
only `calc` sets and only `calc` reads; `PartialEq` now distinguishes a rewritten row from its
original, which is what the rewrite means.
