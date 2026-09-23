# Transactions

The ledger: every operation, newest first, grouped by month with a running net for each. This is
the screen where the portfolio's raw truth lives — every figure elsewhere in the app is derived
from these rows.

**The account picker applies.** A scope narrows what is listed; a transaction has two sides — the
instrument side and the cash side — so an operation is still shown when either side falls inside
the scope.

**Filters.** Search matches instrument, kind, note or account name and runs over the rows already
loaded. The kind and year filters are applied by the app itself, which is why they also change the
monthly totals and the net in the summary line — a search does not.

**Adding a row by hand.** "New transaction" opens a form: account, kind, date, and then whatever
the kind actually needs — a purchase wants a quantity and a price, a fee wants an amount. A
purchase defaults to a securities account because a cash account cannot hold instruments. When the
transaction's currency is not the portfolio's base currency the form asks for the exchange rate of
that day, and that rate stays fixed on the transaction afterwards: later moves in the market do not
rewrite history.

**A commission or a tax in another currency.** Beside the commission and the tax the form has a
currency box each, and leaving it empty means the operation's own currency — which is the usual
case. Filling it records what the broker actually billed: a commission taken in dollars on a euro
trade leaves the dollar balance, not the euro one, and is converted at the rate of its own
currency on that day. It still counts as part of what the shares cost, so a purchase commission
is never also listed among the fees paid.

**Export** writes the operations currently listed — the account picker and the filters apply —
as the app's own transaction file. It is the file the Import screen reads back without asking
anything, and it names accounts and instruments rather than internal identifiers, so it can be
carried to another portfolio or another machine. It holds operations only: it is an interchange,
not a backup of the whole profile.

A whole broker export does not belong here — that is the Import screen, which reads the file,
matches the instruments and writes the rows in one pass.

**Editing and deleting.** Each row's menu edits or deletes it. Rows can also be picked with their
checkboxes and deleted together; deletion is immediate and there is no undo, so the selection bar
states how many rows it is about to remove.

Changing a transaction changes everything computed from it: holdings, cost basis, returns, the
allocation. That is the point of the screen, and also the reason a careless edit is felt far from
here.
