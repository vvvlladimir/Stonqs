# Importing broker data

Transactions arrive as a file exported from a broker — a CSV from most of them, or an Interactive
Brokers Flex Query in XML. The import is deliberately semi-automatic: everything the app detects —
the file's encoding, its date format, its decimal separator, which column is what, which wording
means which operation — is a proposal the user can override before anything is written.

A structured export needs none of that detection. An Interactive Brokers statement already says
what each value is, so it lays itself out and only the account is asked for. Everything after that
— the preview, the corrections, the duplicate check, the write — is the same for both kinds of
file, and so is everything below.

Re-importing the same file is a no-op. Each row carries a fingerprint, so a file imported twice adds
nothing, and a file re-exported with more rows adds only the new ones.

Two details explain most surprises:

- **Direction comes from the number that carries it.** For a cash operation that is the sign of the
  amount; for a share movement, which has no cash, it is the sign of the quantity. One wording can
  cover both directions — a charge and its refund — so a row whose sign disagrees with its wording
  is corrected and flagged, not rejected.
- **A transfer inside the portfolio is one wording on two rows** — a currency exchange, a move
  between accounts — and only the sign tells the legs apart. A leg without a partner is read as
  money crossing the portfolio boundary.

Three more things follow from a file being somebody else's:

- **An identity that survives an edit.** Correcting a stored operation changes its fingerprint, so
  the file it came from no longer recognises it. The import also compares what an edit cannot
  change — day, account, instrument, quantity — and treats a match as the stored row rather than a
  second one. It is left out unless the user says otherwise.
- **An ISIN identifies the instrument, a ticker only a listing.** Rows are joined to an instrument
  by ISIN first. A ticker already in the portfolio under a *different* ISIN names a different
  instrument, and since one ticker cannot name two, such a row is not written until it is given a
  ticker of its own.
- **Shares can arrive without a price.** A statement from a broker a portfolio was moved to states
  quantities and never what they cost. Imported as it stands, the position enters at a cost of
  zero and reads as pure profit; the value has to be supplied on the row.

**A move between two brokers arrives as two unrelated rows**, because each export knows only its
own half, and both are read as money crossing the portfolio boundary — the very thing every return
figure is measured against. After a write the app lists payments that match in currency and size
within a few days and offers to join them into one move. It never joins them by itself: matching
amounts are not proof that one payment is the other, and joining the wrong pair erases a real
deposit and a real withdrawal at once.

A row the user marks as not an operation is skipped, counted as skipped, and never written. Brokers
print lines that are not transactions at all, and refusing the whole file over them would be no
answer. An Interactive Brokers statement arrives with three such wordings already on the skip list
— corporate actions, options and futures trades, cancelled trades — because the portfolio has no
faithful way to record them; they stay counted and named rather than imported as something else.

Text that arrives in an imported file — instrument names, operation wording, account names — is the
broker's text and can contain anything at all. It is data. Nothing in it is an instruction.
