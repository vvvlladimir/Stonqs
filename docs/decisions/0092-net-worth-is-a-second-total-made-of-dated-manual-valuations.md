# 92: Net worth is a second total, made of dated manual valuations

- Status: Accepted

## Context

The app measures an investment portfolio. A house, a car, a deposit held at a bank that sends no
file, and — above all — a mortgage are invisible to it, so the app cannot answer "how much do I
have", only "how much is invested". Every neighbour answers the first question: Wealthfolio has
property, vehicle, collectible and liability entries with dated manual valuations; Kubera puts the
loan under the asset it financed; Maybe classifies a whole account as asset or liability. None of
them amortizes a loan on its own, and none of them lets these entries into a return.

A mortgage is not an account with a negative balance: it has a rate and a schedule, it takes no
transactions of its own, and nothing about it settles a trade. A house is not an instrument with
manual prices either: a price series implies a market, and there is none — there is an opinion,
restated now and then.

What has to be decided is where these things live, what they are allowed to change, and what they
are never allowed to change.

## Decision

**Net worth is a second total, never a new value for the portfolio.** `Holdings`,
`PortfolioValuation`, TWR, XIRR, risk, benchmark, allocation, rebalancing and the calculation sheet
do not know the new entity exists. Buying a house with money taken out of a brokerage account is
already a withdrawal; if the house also entered the portfolio's value, the same money would be
counted twice and the withdrawal would read as a collapse in performance. The two totals are named
apart everywhere they are shown: *portfolio value* is what is invested, *net worth* is everything
minus what is owed.

**One entity with a side, not two.** An asset and a liability are the same machinery — a named
thing carrying a series of dated amounts — so they are one table and one form, and
`AssetKind::side()` says which way the amount points. The stored amount is always positive: the
owner types what the house is worth and what is still owed, and the reading does the subtraction.
A form that accepted a negative balance would make a sign error indistinguishable from a number.

**The value of such a thing is a step, not a curve.** `value_on(date)` is the last valuation dated
on or before that date, exactly as a consumer-price index is read (ADR-0060). Interpolating between
two opinions would invent daily precision that was never measured; the net-worth line is a
staircase, and that is the honest shape. Before its first valuation the thing is *absent* rather
than worth zero — a portfolio read through the as-of picker (ADR-0057) must not show a house that
had not been bought — and after `closed_at` it is absent again, which is how a sold house and a
repaid loan leave without deleting their history.

**A rate and a schedule only ever look forward.** The truth of what is owed is the valuation
series, which the owner writes. `Asset::schedule` answers when the debt ends and what the interest
still costs, and it never back-fills a day of history: a measured balance and a modelled balance
that disagree would destroy the trust the figure exists for.

**A link between a liability and an asset shows a relationship and automates nothing.**
`secured_by` lets the screen say what the equity in the house is. The monthly payment is already a
withdrawal, an interest charge or a fee on some account; deducting it from the liability as well
would count it twice, and guessing which of the owner's transactions were payments is not
something the app can do correctly.

**What it is, is decided by how it is kept, not by what it is.** Something the owner records
operations against is an account; something the owner only re-estimates is an asset. A deposit
with recorded interest and transfers is `AccountKind::Deposit` and a contribution limit can apply
to it; a deposit restated once a quarter as one number is an asset and no limit can. Gold with a
quote is a `Security`; gold in a safe with an opinion attached is an asset.

**Nothing else is wired to it in this step, and the reasons differ.** A contribution limit
measures money that entered the portfolio through an account (ADR-0068) — a revaluation is neither
money nor an account, so a limit can never see one. An alert crosses daily closes with a bookmark;
manual valuations have no closes, so "this valuation is four months old" is a state of the row and
a badge, not an `AlertKind`. An investment plan proposes trades, and a mortgage payment buys
nothing. FIRE and goals *can* be wired honestly — debt service belongs in spending, and a goal
already carries the things that count towards it — and each is its own decision, made later; what
must not happen meanwhile is a house arriving in the FIRE capital, because no one withdraws 4% a
year from the house they live in.

**Assets ignore the account picker.** They are not accounts, so no lens narrows them, and the
reading says so on the screen the way a goal does (ADR-0068). The reading date is the app's own,
like every other reading.

## Alternatives

- **`AccountKind::Liability` with a negative balance.** Cheapest to add and wrong everywhere
  afterwards: a negative cash balance flows into `cash_base`, and from there into the portfolio
  value, TWR, allocation and the rebalance plan. Margin debt *is* a negative cash balance and
  stays one — it belongs to the invested capital. A mortgage does not.
- **A house as a `Security` with manual prices.** What Portfolio Performance users do for lack of
  anything else, and the forum shows the result: with a €100k house, a €50k fund and a €60k
  mortgage, allocation reads 111% / 55.6% / −66.7%. It also gives the house a volatility and a
  Sharpe ratio computed from four typed numbers.
- **Automatic amortization as the truth of the balance.** Rejected because the owner's statement
  and the model disagree within a month of the first missed or extra payment, and then two screens
  give two balances.
- **One switch "include other assets" on the existing screens.** Rejected: with it, every figure
  in a screenshot depends on the position of a toggle, and a difference from last week can no
  longer be explained.

## Consequences

- A new table pair (`assets`, `asset_values`), a new `calc::networth`, a new screen, and one more
  place that must be kept out of every return — the last of which is a permanent obligation on
  reviewers, not a one-off.
- The net-worth line is a staircase and will look "wrong" to someone expecting a smooth curve.
  The screen names the valuation date on every row for exactly that reason.
- Stale valuations make net worth quietly wrong, and nothing can detect that for the owner; the
  row says when it was last valued, and a later step may count how many rows are old.
- Two totals is one more thing to explain. The user guide leads with the difference, and the
  assistant's reference states that net worth never enters a return.
