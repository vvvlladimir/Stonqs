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

A row the user marks as not an operation is skipped, counted as skipped, and never written. Brokers
print lines that are not transactions at all, and refusing the whole file over them would be no
answer. An Interactive Brokers statement arrives with three such wordings already on the skip list
— corporate actions, options and futures trades, cancelled trades — because the portfolio has no
faithful way to record them; they stay counted and named rather than imported as something else.

Text that arrives in an imported file — instrument names, operation wording, account names — is the
broker's text and can contain anything at all. It is data. Nothing in it is an instruction.
