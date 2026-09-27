# Allocation

How the portfolio divides across a **classification tree** — asset class, region, sector, or any
tree the user builds. The account picker applies: the shares are of what is in view, and so does
the date picker: the shares are of the day in view, today unless it was moved back.

Three trees ship ready (asset class, region, sector) and are ordinary user data from then on: they
can be renamed, edited or deleted, and they are never re-translated by the app.

**Views of the same split**, chosen in the strip: a map (area by share, so the large and the small
are visible at once), a tree, rings, one bar, and *target* — where each branch stands against its
target weight.

**Drilling in.** Clicking a branch descends into it; at the bottom the panel lists the actual
instruments and cash under that branch, with the share of each that counts toward it.

**A subject can be split across branches.** An instrument is not forced into one bucket: it can be
60% one region and 40% another, and the panel shows the assigned share. Cash counts as a subject
too, one per account *and* currency, so one deposit account holding euros and dollars is two.

**Unclassified is a real bucket**, and a dot on the tree's tab says something is in it. Percentages
divide by what is classified, so leaving instruments unclassified quietly changes every weight —
that is why the screen points at it.

**Disabling a subject is not deleting it.** A subject switched off for a tree leaves the charts and
the percentages entirely, but it is still listed, dimmed, and its classification stays untouched
for when it is switched back on. It is per tree: off in "Region" does not mean off in "Sector".

**Filling a tree.** A tree can be classified by hand, imported from a CSV (which extends the tree
rather than replacing it — a branch already there under the same name is reused, so re-importing
does not double it), or seeded from an instrument attribute: one branch per distinct value, each
instrument carrying that value assigned whole. What a tree already classifies by hand is left
alone — a typed split outranks a column. A tree can be exported to CSV as well.

A plugin can bring a **ready classification set** — a tree already laid out, offered by name in the
same two places a CSV is: on a tree's own menu, where it extends that tree, and beside
`Import from CSV…` when no classification exists yet, where it creates one. It goes through the
same preview and the same write as a file, and a set that matches none of the portfolio's
instruments still gives the tree, leaving the assignments to be made by hand.

Targets are edited here and used on the Rebalance screen.
