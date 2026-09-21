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
3. **Instruments** — link the export's codes to real instruments so that prices can arrive
   afterwards. A code the broker uses that matches nothing yet can be resolved here.
4. **Commit** — the summary, then the write.

**Saved layouts** answer every mapping question at once for a broker the user imports from
regularly. A layout is applied to the *next* export, not the one it was built from, so the app
still fills in wordings the layout does not mention. Picking a layout is undoable: what the app
detected on its own is kept.

Rows carry a status. An error blocks that row alone, never the file; a warning — a corrected
direction, a suspicious amount — is shown and imported. The wizard refuses to continue only when a
required column has not been pointed at.

**Re-importing the same file adds nothing.** Rows are recognised by their content, so a monthly
export that repeats the previous month is safe. There is an option to import duplicates anyway,
which exists for the case where two identical operations really did happen.

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
