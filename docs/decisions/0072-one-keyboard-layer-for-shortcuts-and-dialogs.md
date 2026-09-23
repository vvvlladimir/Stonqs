# 72: One keyboard layer for shortcuts and dialogs

- Status: Accepted

## Context

The app had no shortcuts, and every overlay listened for `Escape` on `document` by itself (`Modal`,
the assistant panel, the navigation sheet). One `Escape` therefore closed every open layer at once,
and a dialog neither moved focus into itself nor gave it back. Adding shortcuts screen by screen
would have made that worse: each listener deciding for itself whether a key was meant for it, with
no way to turn the bare-key ones off, as WCAG 2.1.4 requires.

## Decision

`lib/commands/` is the command layer, imported only through its barrel: `catalog.ts` (what the app
can do), `keys.ts` (bindings), `registry.ts` (who answers now, as plain data with `subscribe`) and
`react.tsx` (the only window-level `keydown` listener and the hooks).

- `COMMANDS` is the one catalogue of commands, with or without a key. A binding is `mod+k` (⌘ on macOS, Ctrl
  elsewhere), a bare key (`n`, `?`, `[`) or a two-key sequence (`g p`). The palette, the shortcut
  list, tooltips, `aria-keyshortcuts` and the macOS menu bar all read it, so a binding is changed in
  one place.
- A component answers a command with `useCommand` / `<Command>` while mounted. A screen's answer
  outranks the shell's, so `mod+n` on Transactions opens the form in place. A command whose catalogue
  row names a `screen` needs no shell handler: run anywhere else, the registry opens that screen and
  leaves an intent that the screen's `<Command>` consumes once it is mounted (5 s at most).
- A set of exclusive options (period, data source, colour scheme, profile) is a **choice**,
  published with `useChoice` by whoever owns it. The menu bar draws it as checkable items and the
  palette lists its options, so neither knows where a period lives.
- An overlay pushes a layer with `useLayer`. `Escape` reaches only the newest layer, and while a
  layer is open no command runs unless the layer lets it `pass`. `useDialogFocus` gives every modal
  the WAI-ARIA dialog behaviour: focus moves in, `Tab` stays in, and focus goes back on close.
- A letter is matched by what it types on a Latin layout and by its physical position on any
  other, so the shortcuts work with a Cyrillic layout active. Bare keys never fire while typing or
  inside a menu, and `UiState::shortcuts.single_keys` turns them all off.
- On macOS the menu bar is data (`components/domain/menuBar/model.ts`): entries name commands,
  choices, native items or the navigation's own screen lists. `MenuBar` resolves it against the
  registry, so an item is greyed out exactly when nothing would answer it. `api::setAppMenu`
  rebuilds only when the shape changes and otherwise updates `enabled` / `checked` in place.
  Labels are Lingui's like any other text, and the OS keeps its own items (edit, hide, quit). One
  keystroke seen by both the menu and the webview runs once.

## Alternatives

- A library (`tinykeys`, `react-hotkeys-hook`): it matches keys but has no idea of layers,
  priorities or the menu bar, which is where the actual work is.
- A menu built by the host in Rust: its labels would be sentences crossing IPC (ADR-0023).
- Shortcuts registered by each screen with its own listener: that is what broke `Escape`.

## Consequences

- A new overlay calls `useLayer` (or uses `Modal`). It does not listen for `Escape` itself.
- A new shortcut is a row in `COMMANDS` plus one `useCommand`. The list and the palette pick it up
  without further changes; a menu item is one more line in `menuBar/model.ts`.
- Windows and Linux have no menu bar, so everything in it must also be reachable from the palette.
  Building both from the registry keeps that true by construction.
- The user guides describe the Keyboard settings panel but list no keys (`assistant-docs.md`).
  The in-app list is the reference.
