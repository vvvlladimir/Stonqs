# Cost basis and lots

A holding is not one blob of shares; it is the purchases that produced it, each with its own date,
price and costs. A sale consumes those lots, and which lot it consumes decides the gain.

Two methods exist and the portfolio picks one:

- **FIFO** — the oldest lot goes first. This is the default and what most European tax regimes
  assume.
- **Average cost** — all shares are one pool at a weighted average price. A purchase moves the
  average; a sale does not.

Buy commission is part of cost basis: it raises what the shares cost, and it is therefore *not*
also counted as a standalone fee. Standalone fees and taxes — account charges, custody fees, a
withholding tax on a dividend — are separate operations and are counted as charges.

That distinction matters when the question is about costs. "What did this position cost me to buy"
includes the commission that went into the basis; "what did I pay in fees this year" as shown on
the income and costs screens counts standalone charges. A cost *rate* — what share of the portfolio
was spent on costs — counts both, because it asks the opposite question.

Unrealised result is today's value against the basis of the lots still held. Realised result is the
result of the lots a sale consumed, at the prices those lots were bought at.

## Reading both methods at once

Every figure the app computes uses the method the portfolio is set to. The positions table can
nonetheless show the purchase value, the purchase price, the unrealised and the realised result
under **both** methods side by side, each column named after its method, so the choice can be
compared instead of assumed.

The two only ever disagree about a position that was **partly sold**: until then there is one pool
and one queue holding the same shares at the same prices. After a partial sale, FIFO has spent the
oldest lots and average cost has spent an average share, so what is left cost a different amount
under each — and what the sale realised differs by exactly the same amount in the opposite
direction. Over the life of a position the two totals therefore agree; they disagree about when
the result is counted, which is why the choice is a tax question rather than a valuation one.

Quantity, price and market value are the same under either method. So is anything derived from
the value series — time-weighted return, volatility, drawdown — because those never read a cost.
