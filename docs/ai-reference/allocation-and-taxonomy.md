# Allocation and classification trees

An allocation answers "how is the portfolio divided" against a **classification tree** the user
owns. Three trees ship — asset class, region, sector — and the user may rename them, edit them, or
build their own. A tree's name is the user's text, so always ask for a breakdown by the name the
app lists, never by a name invented for the answer.

What gets classified is a **subject**: either an instrument, or the cash of one account in one
currency. A deposit account holding both EUR and USD is two subjects, because the two behave
differently in a currency breakdown.

An instrument can be split across several nodes with weights — a world ETF is part US, part Europe,
part Asia. Weights are shares of that instrument, and they are the user's own numbers.

Anything a tree does not classify falls into an explicit unclassified bucket. It is part of the
total and it is visible: a chart that quietly dropped it would misstate every other share.

A subject can be **excluded** from one tree. An excluded subject leaves that tree entirely — out of
the denominator, out of the chart, out of the percentages — and is not the same as being
unclassified. Exclusion is per tree, it is reversible, and it never touches the classification
itself. Typical use: excluding a pension account from the region breakdown while keeping it in the
asset-class one.

Because exclusions are per tree, two trees can legitimately report different totals for the same
portfolio. That is not an inconsistency to reconcile.
