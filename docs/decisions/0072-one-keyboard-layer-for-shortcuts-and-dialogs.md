# 72: One keyboard layer for shortcuts and dialogs

- Status: Accepted

## Context

The app had no shortcuts, and every overlay listened for `Escape` on `document` by itself (`Modal`,
the assistant panel, the navigation sheet). One `Escape` therefore closed every open layer at once,
and a dialog neither moved focus into itself nor gave it back. Adding shortcuts screen by screen
would have made that worse: each listener deciding for itself whether a key was meant for it, with
no way to turn the bare-key ones off, as WCAG 2.1.4 requires.

## Decision

`lib/shortcuts.tsx` is the only window-level `keydown` listener.

- `COMMANDS` is the one catalogue of what a key does. A binding is `mod+k` (⌘ on macOS, Ctrl
  elsewhere), a bare key (`n`, `?`, `[`) or a two-key sequence (`g p`). The palette, the shortcut
  list, tooltips, `aria-keyshortcuts` and the macOS menu bar all read it, so a binding is changed in
  one place.
- A component answers a command with `useCommand` / `<Command>` while mounted. A screen's answer
  outranks the shell's, so `mod+n` on Transactions opens the form in place. A command pressed on a
  screen that is not mounted yet leaves an intent for that screen to read when it opens.
- An overlay pushes a layer with `useLayer`. `Escape` reaches only the newest layer, and while a
  layer is open no command runs unless the layer lets it `pass`. `useDialogFocus` gives every modal
  the WAI-ARIA dialog behaviour: focus moves in, `Tab` stays in, and focus goes back on close.
- A letter is matched by what it types on a Latin layout and by its physical position on any
  other, so the shortcuts work with a Cyrillic layout active. Bare keys never fire while typing or
  inside a menu, and `UiState::shortcuts.single_keys` turns them all off.
- On macOS the frontend builds the menu bar (`api::setAppMenu`) from the same catalogue, with
  labels translated by Lingui like any other text, and keeps the OS's own items (edit, hide, quit).
  One keystroke seen by both the menu and the webview runs once.

## Alternatives

- A library (`tinykeys`, `react-hotkeys-hook`): it matches keys but has no idea of layers,
  priorities or the menu bar, which is where the actual work is.
- A menu built by the host in Rust: its labels would be sentences crossing IPC (ADR-0023).
- Shortcuts registered by each screen with its own listener: that is what broke `Escape`.

## Consequences

- A new overlay calls `useLayer` (or uses `Modal`). It does not listen for `Escape` itself.
- A new shortcut is a row in `COMMANDS` plus one `useCommand`. The list, the palette and the menu
  pick it up without further changes.
- The user guides describe the Keyboard settings panel but list no keys (`assistant-docs.md`).
  The in-app list is the reference.
