# Rebalance

Compares the portfolio against a **target** — a set of weights over the nodes of one classification
tree — and proposes trades that would close the gap. It proposes: nothing is written by this screen
except when the user explicitly turns the result into a savings plan.

Without a target there is nothing to compare against, and the screen says so rather than inventing
one. A portfolio can have several targets over different trees; the control picks between them.

**Mode** is the important switch:

- *Buy and sell* — the plan may trim overweight nodes to fund underweight ones.
- *Buy only* — overweights are left alone and the money available is split across the shortfalls in
  proportion to their size. This is the usual choice: selling realises a result and can be a taxable
  event, which is a bigger decision than the reshuffle itself.

**New money** is entered on the screen and is added to the portfolio total *before* the targets are
applied, so the plan spends it instead of assuming it is already invested. The summary shows the
portfolio including it.

The figures at the top: the portfolio total the targets are derived from, the largest drift in
percentage points and which node it is, how much to buy against how much to sell, and the number of
trades with whatever money is left over.

**Quantities round down to the instrument's tradable step.** A third of a share cannot be bought, so
the leftover is real money that stays money, reported rather than hidden.

The deviation chart reads: the fill is what is held, the tick is the target, hatching past the tick
is an overweight, a dashed run up to it is a shortfall.

The buys can be turned into an investment plan in one action — the same split, as a standing monthly
contribution, so the decision is made once rather than every month. Sales are dropped from it: a
contribution pays money in.
