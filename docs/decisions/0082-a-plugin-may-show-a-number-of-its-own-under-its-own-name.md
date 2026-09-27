# 82: A plugin may show a number of its own, under its own name

- Status: Accepted; narrows how "never changes what a number means" in
  [ADR-0070](0070-a-plugin-brings-data-and-shows-it-it-never-changes-what-a-number-means.md) is read

## Context

ADR-0070 says a plugin brings data and draws it, and never changes what a number means. Read
literally that forbids a plugin to compute anything, and the plugins already shipped do compute: a
file reader turns a statement into amounts, a writer lays operations out as balanced journal
entries. A dashboard widget that may show only figures the app already shows elsewhere is a
restyle, not an extension — a heat map of positions coloured by return is something no built-in
tile draws, and it has to divide one figure by another to draw it.

What ADR-0070 protects is narrower than "no arithmetic": two users of this app must be able to
compare a TWR, a cost basis or a position value and know that both came from the same code. A
plugin that silently replaced one of those would break that. A plugin that shows a figure the app
does not have, and says whose it is, breaks nothing.

## Decision

- **Forbidden: overriding a figure the app answers for.** Lots, cost basis and its methods,
  holdings and valuation, TWR and XIRR, the risk metrics, the flow rules and the scope rewrite, the
  FX chain. A plugin gets these as the app computed them and cannot hand back a different value
  under the same name — there is no channel through which a plugin's figure reaches another
  screen, a report, an export or the assistant.
- **Allowed: a number of its own, under its own name.** Anything a plugin derives from what it was
  given — a ratio, a score, a grouping, a projection — is the plugin's. It is shown only inside
  what the plugin draws, and everything a plugin draws carries the plugin's name, so the user can
  always tell which figures are the app's and which a stranger computed.
- **Nothing a plugin computes is stored.** A plugin's figure is recomputed from the app's own data
  each time it is shown; it never becomes an input to a calculation of the app's.

## Alternatives

- **Keep the literal reading.** Leaves UI plugins with nothing to show that a built-in tile does
  not already show, which makes the whole UI kind pointless.
- **Let a plugin register a metric the app then treats as its own** (in the metric catalogue,
  exports, the assistant). The most useful and the one that makes "which code produced this
  number" unanswerable; it would need a trust model ADR-0070 deliberately left open.

## Consequences

- A plugin's tile shows the plugin's name in its header, always; that is part of this decision,
  not a styling choice.
- A figure a plugin shows can disagree with a figure the app shows (it may round, or define a
  "return" differently). The user guide says whose number is whose rather than promising they agree.
- A future request to let a plugin feed a figure back into the app is a new ADR, not an
  extension of this one.
