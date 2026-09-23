# Plans

Standing contributions: how much, how often, from which account, and what the money buys. A plan
describes an intention — it never writes a transaction on its own.

**Plans ignore the account picker.** Narrowing the scope must not hide next month's savings, so
this screen always shows every plan of the portfolio.

Each plan card shows the amount and its currency, the cadence, the account it is paid from, the
next date it fires, and how the contribution is split across instruments — one bar with a legend,
because a list of tickers does not say what fraction each one gets. A plan with no instruments at
all is a cash contribution: the money simply stays on the account. A stopped plan keeps its card
and its history; it just stops proposing.

The summary line adds up what every active plan contributes in an average month, converted at
today's rates.

**Due contributions are the point of the screen.** When a scheduled date has passed with nothing
recorded against it, a banner says so and the plan's card offers a review. The review panel shows
one occurrence at a time with the transactions the plan proposes — quantities worked out from the
price on that date — and **nothing is written until Record is pressed** for that occurrence. The
numbers can be typed over first, which is the normal case: the broker filled a slightly different
price, or charged a fee the plan did not know about.

An occurrence whose date has no price yet says so rather than proposing a quantity it had to
invent.

Recording an occurrence writes its transactions and links them to that date, so the plan stops
offering it. Deleting one of those transactions later offers the month again by itself — the link
is what says "already done", not a flag on the plan.

Deleting a plan leaves every transaction it ever recorded where it is. It removes the intention,
not the history.

**Goals** sit below the plans and answer a different question: an amount you mean to have by a
date. A goal names the accounts that count towards it — none ticked means the whole portfolio — and
it ignores the account picker entirely, so it reads the same wherever you happen to be looking.

Give it a date and it says what would have to go in each month to arrive on time. Give it a monthly
amount instead and it says when that pace arrives, or that it does not. Give it both and it also
says whether you are on track; with only one of the two that question has no answer, and the goal
says nothing rather than calling you behind.

The expected return is an assumption you type, not what this portfolio has actually returned.
Leaving it empty is the honest default for a savings goal: the money grows only by what is paid in.

Deleting a goal changes nothing else. It is an intention, not a record.
