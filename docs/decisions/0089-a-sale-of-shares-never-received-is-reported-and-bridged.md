# 89: A sale of shares never received is reported and bridged, not fatal

- Status: Accepted

## Context

A user imported a Trade Republic export in which crypto was received by transfer
(`FREE_RECEIPT`, a wording the shipped layout did not know) and then sold. The receipts were not
written, the sales were, and `take_lots` answered the first sale with `Error::Invalid("disposal of
<uuid> with no open position")`. Every reading of the portfolio runs the same holdings pass, so
every tile of the dashboard, the performance screen and the reports showed that one sentence —
naming an id, not an instrument — and nothing else. The import had said nothing: an unknown
wording is an error on *its* row, and a sale is only wrong against the rest of the ledger.

A second defect made the same failure random: transactions of one day are read `ORDER BY date,
id`, and the id is a random UUID, so a receipt and a sale on the same day met in either order.

## Decision

- `calc::quantity_gaps` replays the ledger in the holdings builder's own order and lists every
  disposal (sale, outgoing delivery, outgoing security transfer) that takes more than was held,
  with what was held and what is missing.
- `PortfolioAnalytics::transactions_until` bridges each gap with an **implied inbound delivery**
  of the missing quantity at the disposal's own price, placed just before it. Its result is
  therefore nil and its value enters as an external flow, so a missing purchase costs one
  instrument's accuracy instead of every figure, and TWR is not bent by proceeds from nowhere. The
  row is never stored and never listed. `build_holdings` itself still refuses such a ledger: a
  caller that did not ask for a bridge does not get one.
- `portfolio_gaps` hands the gaps to the frontend over the **whole portfolio** — a hole is in the
  ledger, not in a view — and a banner above every screen names the instrument and says its
  figures are estimated until the purchase or incoming transfer is added.
- The import replays the stored ledger together with the rows about to be written and puts
  `SaleExceedsHoldings` on each uncovered disposal, naming the file's own unread rows of the same
  instrument (an unmapped wording, a value marked "do not import") as the likely cause. A warning,
  like every plausibility check: the purchase may be in the next file.
- Within a day, acquisitions (`Buy`, `DeliveryInbound`) are applied before anything else, then
  security transfers out and in, then the rest in storage order.

## Alternatives

- **Keep the hard error, name the instrument.** Readable, but the whole app still shows nothing
  until the user finds and fixes one row — typically in a file they cannot re-export.
- **Skip the uncovered part and keep the cash.** Proceeds would appear from nowhere as a return.
- **Bridge at a cost of zero.** The whole proceeds would read as a realised gain.
- **Make the import refuse the row.** The purchase may legitimately arrive with the next file, and a
  check that blocks on a false positive teaches users to work around checks.

## Consequences

- Every engine reading succeeds on such a ledger, and every one of them is an estimate for that
  instrument until the banner goes away; the banner is the only thing saying so.
- The realised result of the uncovered part is the sale's charges only; the implied inflow shows
  in "Value and flows" as money arriving.
- Same-day ordering changes the average cost a same-day sale consumes under the average-cost
  method; it was random before.
