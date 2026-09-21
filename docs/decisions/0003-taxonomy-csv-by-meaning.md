# 3: Read taxonomy CSVs by meaning


- Status: Accepted

## Context

Taxonomy exports use different header names and may include a repeated tree
name, category rows, security rows, targets, and per-category security
weights. Supporting one vendor template would make imports brittle.

## Decision

Detect columns from header meaning rather than fixed positions. Numbered level
columns are ordered numerically; named category columns are used as a fallback.
A row with a ticker or ISIN is a security assignment, and a repeated first
level is treated as the tree name rather than a category. Securities are
matched by ISIN before ticker, because an ISIN identifies the instrument while
a ticker identifies one listing.

Taxonomy preview is pure. Commit extends an existing tree, reuses matching
nodes case-insensitively, and replaces a security's newer split assignment.

## Consequences

Exports from different applications can be imported without vendor-specific
templates. Re-importing the same file does not duplicate the tree, while the
preview still exposes unmatched securities and malformed weights.
