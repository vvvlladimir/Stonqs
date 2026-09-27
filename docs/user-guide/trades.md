# Trades

A trade is a round trip — money going into shares and coming back out — not an instrument and not
a transaction. How the round trips are counted is the switch beside the period, and the two
answers are both right; they answer different questions.

**Per position** (the default): every sale closes one trade, made of the purchases that sale used
up, and whatever is still held of an instrument is one open trade. Someone who bought a fund every
month for a year and sold nothing has **one** open trade in it, its age averaged over the
purchases by quantity. This answers "how did holding this go".

**Per purchase**: every purchase is a trade of its own. A sale that used up three purchases closes
three trades, and its proceeds — after its fees and tax — are shared between them by quantity; a
purchase only partly sold is split into a closed part and an open part. The twelve monthly buys are
twelve trades. This answers "how did each purchase go". The results add up to the same total
either way — the split changes, the money does not. Under average cost the purchases are merged
into one, so both switches show the same thing.

The period selects which trades were **closed** inside it. What is still held is shown as of the
end of the period instead, because an open trade has no closing date to fall inside a window.

**Closed in the period** lists each finished trade: when it was entered and exited, how long it was
held, what it cost and what it brought in, the result, and an IRR. Entry is what the shares cost
*including* the purchase commission; exit is what the sale brought in *after* its own fees and tax.
The IRR dates those two amounts, which is why a quick double and a slow one stop looking alike —
the same 100% over three months and over three years are not the same trade.

**Still open** is the other half: what is still held, valued at the end of the period — one row per
instrument per position, one per remaining purchase per purchase.

The headline figures cover both sides and follow the switch — how many trades closed, the hit rate, the result of the
closed ones, the average holding period, how many are still open and what they are worth, and the
turnover. Only the first few of these figures are on screen; the rest are one click away behind a
control reading `more figures`, and `Show less` puts them back. Nothing is dropped — a figure that
is not visible has simply not been unfolded.

Nothing closed in the period does not mean nothing was sold ever: a trade belongs to the period its
sale fell in, so widening the period is usually the answer. The dashboard's trade tiles always
count per position.
