# 84: A plugin screen, and the one document a plugin keeps

- Status: Accepted

## Context

ADR-0083 put a stranger's page on the dashboard: a frame with no origin, handed the reads its
manifest declared. The next surface is a whole screen, and the test ADR-0070 set for it is a real
one — spending and categorisation, written as a plugin. That plugin needs two things a tile did not:
the **operations** themselves (a bank statement read by the MT940 reader is withdrawals with notes),
and somewhere to **keep its rules**, because "Supermarket → Groceries" typed once must still be
there tomorrow.

The navigation is a closed set today. `ScreenId` is a union the command catalogue, the tour, the
assistant's guides (`guide::SCREEN_IDS`) and the menu bar are all checked against, and the user's
arrangement of it is stored as bare ids (`UiState::nav`).

## Decision

### A screen is a larger widget

- `provides.screens: [{ id, name, description, file, reads, periodic, storage }]`. The page is
  served exactly as a widget's — same scheme, same shim, same policy, same frame — under
  `/screen/<plugin>/<id>` beside `/widget/<plugin>/<id>`.
- A screen follows the **app's** lenses, like every built-in screen: the account picker and the
  date. A `periodic` screen gets the app's period control in its header, and its period reads are
  answered for that period.
- The host draws the page around the frame — title, the plugin's name, the period control — and the
  frame fills the rest. The plugin's name is always shown, for ADR-0082's reason.

### Navigation gains one screen, not an open set

- `ScreenId` gains `plugin`; *which* plugin screen is the navigation hint (`go("plugin", key)`),
  the mechanism that already carries an instrument to the Securities screen. Every closed list stays
  closed, the assistant is told the user is on a plugin's screen, and the guide for it says the app
  cannot explain what a stranger's screen shows.
- Plugin screens are listed in a **Plugins** section after the shipped ones, in the plugin list's
  order, and in the command palette. They are not pinned, not reordered and not in the macOS menu
  bar in this decision: the stored arrangement stays a list of ids the app itself defines, so a
  removed plugin can never leave a hole in it.

### `transactions` is a screen's read

- A new read, `transactions`: the operations of the app's scope over the screen's period, each
  with an opaque `id` stable for as long as the row exists — without one, a plugin could remember a
  choice about a row only by guessing at its content. It requires `periodic`.
- It is **not** offered to a widget. A widget reads through its own data source, and the operation
  list has only the app's lens; a widget that silently ignored its own source would be answering a
  different question from the one on its settings.

### One document per plugin, in the profile

- A plugin declaring `storage` keeps **one JSON document** in the open profile (`plugin_state`,
  migration 0031), handed to it as `data.state` on every render and replaced whole by
  `stonqs.save(doc)`. In the database rather than beside it, so an encrypted profile encrypts it,
  and a locked one refuses it like every other read.
- It is the plugin's own data, not the portfolio's, so saving it asks nothing: it cannot change an
  operation, an account or a figure, and nothing in the app reads it. That is the line between
  this and `Write`, which still asks every single time and which no plugin has.
- A document is capped at 256 KiB. Removing a plugin leaves its document in place: uninstalling is
  not deleting what the user typed, and reinstalling finds it again.

## Alternatives

- **Open `ScreenId` to any string.** Every closed check in the frontend and the host becomes a
  runtime check, and a stored favourite can name a screen that no longer exists.
- **Keep the plugin's rules in `localStorage` of its frame.** The frame has no origin and therefore
  no storage, deliberately; loosening that would give every plugin a place outside the profile,
  unencrypted, that follows no profile switch.
- **A key-value store with many keys.** More API for the same thing; one document is what a plugin
  of this size needs, and it is replaced atomically.
- **Offer `transactions` to widgets through the picker's lens.** A tile whose source setting is
  ignored for one of its reads is a trap.

## Consequences

- Spending and categorisation is a plugin (`examples/plugins/spending`), and it needed no hole in
  the core: operations in, rules kept, figures of its own. That is the check ADR-0070 asked for.
- Pinning a plugin screen, or placing it inside a shipped section, needs the stored arrangement to
  learn ids it does not define — a later decision.
- The bridge now has a message from the frame that causes a write, the document save. It is the
  only one, and it can reach nothing but that plugin's own row.
