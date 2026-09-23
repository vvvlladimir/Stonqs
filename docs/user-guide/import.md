# Import

A four-step wizard over one file exported from a broker — a CSV, or an Interactive Brokers Flex
Query in XML. **Nothing is written to the portfolio until the button on the last step.** Everything
before that is a preview that can be changed and recomputed freely.

The steps are four questions:

1. **File** — which export is being read and what is actually inside it. The file's encoding, date
   format and decimal separator are detected here and can be overridden; brokers produce all of
   them, and a file is often not UTF-8.
2. **Parsing** — which column means what, which of the broker's operation words means which kind of
   transaction, and which account everything lands on. Only the date and the kind columns are
   required; the rest improve the result. A wording the user does not want imported at all can be
   marked as skipped rather than mapped to something wrong.

   The account is asked as what the file *is*: a broker statement, or a bank or wallet statement.
   Choosing the first records instruments on the brokerage account and moves the money on the cash
   account linked to it, so there is no second question to answer; choosing the second puts
   everything on one cash account. Only when the portfolio holds more than one account of the kind
   chosen does the step ask which. What was picked is then restated in full — where rows land, and
   where money moves — before anything else on the step.
3. **Instruments** — link the export's codes to real instruments so that prices can arrive
   afterwards. The search starts by itself when the step opens and works down: the ISIN, then the
   code and the name printed in the file, then the exchanges the instrument trades on, taking the
   first one that actually has prices rather than the first that merely has a name. Anything it
   got wrong can be corrected or pointed at another listing, and "Search again" repeats the whole
   pass. A code left unresolved enters the portfolio under the broker's own spelling and stays
   without prices, which is why the step says how many are in that state.
4. **Commit** — the summary, then the write.

**Saved layouts** answer every mapping question at once for a broker the user imports from
regularly. A layout is applied to the *next* export, not the one it was built from, so the app
still fills in wordings the layout does not mention. Picking a layout is undoable: what the app
detected on its own is kept.

**A recognised file lays itself out on opening.** A layout — one the user saved or one shipped
with the app — claims a file by the columns it expects, so an export from a known broker arrives
with its settings already filled in and the layout named at the top of the step. Two layouts that
fit a file equally well recognise nothing: the app would rather ask than pick the wrong broker.
Choosing "— detect —" throws the layout away and reads the file from scratch.

Rows carry a status. An error blocks that row alone, never the file; a warning — a corrected
direction, a suspicious amount — is shown and imported. The wizard refuses to continue only when a
required column has not been pointed at.

**Prices are fetched after the write, as far back as the file goes.** The instruments the import
created and the currencies it introduced are fetched from the date of their earliest operation in
the file, not from a fixed window, so an export covering ten years does not leave its older half
without prices or exchange rates. This runs in the background; the import screen does not wait for
it. An instrument whose prices come back almost empty is moved to another exchange automatically
and fetched again — see the Instruments screen, where such a row is marked.

**Re-importing the same file adds nothing.** Rows are recognised by their content, so a monthly
export that repeats the previous month is safe. There is an option to import duplicates anyway,
which exists for the case where two identical operations really did happen.

**A row the broker restated replaces the one already stored.** When the export carries the
broker's own identifier for each row — a transaction or order number — that identifier decides
what a row *is*. The same identifier with the same values is a duplicate and is skipped; the same
identifier with corrected values is marked as restated, and writing it updates the stored
operation instead of adding a second one. Restated rows are listed before the write and counted
separately afterwards, because nothing new entered the ledger. The duplicate switch does not
cover them: a correction is not a repetition.

**Notices worth reading before the write.** The wizard checks the file for four things it cannot
decide on the user's behalf, and every one of them is a notice rather than a refusal:

- *Shares moved with no value given* — a position carried in from another broker arrives as a
  quantity with no price. Written as it stands, the holding enters at a cost of zero and shows the
  entire position as profit. The row can be opened and the price paid, or the total, typed in —
  even when the file has no such column at all.
- *Another currency than the account keeps* — the row is denominated in something the cash account
  it lands on does not hold. Legal, since an account really can hold two currencies, and also
  exactly what a mis-read currency column looks like.
- *The ticker already names another instrument* — the file's ticker is in the portfolio already
  under a different ISIN. These are two instruments, and one ticker cannot name both, so the row
  is not written until it is given a ticker of its own on the "Instruments" step. This is the one
  notice that does block its row, because writing it would pour one company's trades into
  another's position.
- *The prices step by a whole factor* — one instrument trades at one price and then at a third or
  a tenth of it. If the broker applied a split part-way through the statement, the quantities
  before and after mean different shares; a split is recorded on the instrument rather than
  imported.

**A row edited by hand is not imported twice.** Correcting a stored operation changes what it
says, so the next import of the same file no longer recognises it by content. The app also
compares day, account, instrument and quantity — what an edit cannot change — and marks such a row
as already stored. It is left out by default and can be written anyway with the switch beside it,
for the case where the same size really did trade twice in one day at two prices.

**Money that left one account and arrived at another** is offered for joining after the write.
Two brokers export separately, so carrying a portfolio between them arrives as a withdrawal in one
file and a deposit in the other, and the app reads each as money crossing the portfolio boundary —
which every return figure is measured against. Payments that match in currency and size within a
few days are listed as candidate pairs; confirming one makes it a single move. Nothing is joined
automatically: two amounts agreeing is not proof that one payment is the other.

**Some brokers print the amount with the commission already in it.** The app works out which of
the two a file means by comparing the amount against quantity × price across the whole file, and
says what it decided beside the other parse settings, where it can be set by hand. The
distinction matters: read the wrong way round, every purchase's cost is off by its commission.

**One line of a file can be two operations.** A reinvested dividend is income *and* a purchase;
money moved between two of your own wallets is a leg out and a leg in. Where the broker prints
such a line once, the transaction-kind list offers those two answers beside the ordinary kinds,
and the preview then shows the line twice — numbered `12.1` and `12.2`, one row of the file, two
operations. Both halves are written together or not at all, and a later corrected export still
recognises each half separately.

## The app's own file

The app writes its own transaction file, and reads it back with nothing to answer: the format
states the dates, the numbers, the operations and the currencies in the app's own terms, so the
parsing and mapping steps have no questions left. It is produced by "Export" on the Transactions
screen and holds exactly what that screen was showing.

The file names accounts and instruments the way a person would — by account name, ticker and
ISIN, never by an internal identifier — so it imports into a different portfolio, on a different
machine, and not only back into the one it came from. An account whose name already exists is
matched to it; anything unmatched is asked for in the wizard as usual.

It carries operations and nothing else. Classification trees, investment plans, alerts and
settings are not in it, and a backup of everything is the profile itself rather than this file.

## Interactive Brokers

An Interactive Brokers export is a Flex Query: a report the user defines once in the broker's web
office and then downloads as XML for any period. Such a file is recognised on sight and lays itself
out — no encoding, date format, column mapping or operation wording to answer. The only question
left is which account the statement belongs to.

The Flex Query has to select the right sections, or the parts it leaves out simply are not in the
file: Account Information, Trades, Cash Transactions and Corporate Actions, with currency rates
turned on.

Three kinds of row are deliberately **not** imported and appear on the skip list instead, counted
and named so the choice is visible and can be taken back: corporate actions, options and futures
trades, and cancelled trades. A corporate action is left out because prices already account for a
split and lots are adjusted separately, so importing the broker's quantity change would count it
twice. Options and futures are left out because a contract stands for a number of underlying units
that the portfolio has nowhere to record.

Two things the file does that no CSV does: a currency exchange arrives as a linked pair of
transfers, so it counts as money moving inside the portfolio rather than leaving and returning; and
a dividend and the tax withheld from it stay two separate operations, so the gross payment every
yield figure is based on is not quietly netted away.

Asking one Flex Query for both executions and orders is harmless. When a purchase appears as both
its individual fills and the order behind them, only the order is taken.

Options on the last step decide what happens to instruments the portfolio does not have yet:
whether to create them, what type they get, and which quote provider they start on. A newly created
instrument fetches its prices on its own once the import is written.
