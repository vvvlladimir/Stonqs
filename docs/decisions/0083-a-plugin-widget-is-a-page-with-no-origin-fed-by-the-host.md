# 83: A plugin widget is a page with no origin, fed by the host

- Status: Accepted; who builds the data superseded by ADR-0088

## Context

ADR-0070 decided that UI plugins are ES modules in an iframe with an opaque origin, reaching data
through a host bridge, and left the bridge undefined. The dashboard is the first surface: a tile
already configures itself (`WidgetDef::fields`), is sized on the board's grid and reads its data
through the same scoped queries as every screen (ADR-0029/0030). ADR-0082 settled what a tile may
show. What remains is how a stranger's JavaScript runs inside this window without reaching what the
window reaches — the store through IPC, the network, the other tiles — and how it gets data.

Two facts about the host shape the answer. Tauri injects its IPC bootstrap and the per-launch
invoke key into the **main frame only** (`for_main_frame_only` in `tauri::manager::webview`), so a
subframe has no way to call a command. And the app's own CSP (`default-src 'self'`) would be
inherited by a `srcdoc` frame, forbidding exactly the inline script a plugin is.

## Decision

- **The page is served by the host, not assembled by the frontend.** A custom URI scheme,
  `stonqs-plugin`, answers `/<plugin id>/<widget id>` with one HTML document: a small shim, then
  the widget's module inline. Its own `Content-Security-Policy` header is
  `default-src 'none'; script-src 'nonce-…'; style-src 'unsafe-inline'; img-src data:; font-src data:`
  — no `connect-src`, so no `fetch`, no WebSocket and no request to the IPC scheme either; the nonce
  is new per response. The app's own CSP gains only `frame-src` for that scheme.
- **The frame is `sandbox="allow-scripts"` and nothing more**: an opaque origin, no storage, no
  popups, no forms, no navigation of the window. The only channel is `postMessage`, and the host
  accepts a message only from that tile's own frame (`event.source`).
- **Data is pushed, never asked for.** The manifest declares what a widget `reads`, from a closed
  list — `valuation`, `positions`, `performance` in API 1. The host fetches exactly those, through
  the same query hooks and keys as the built-in tiles, under the tile's own data source and period,
  and posts them with the context (date, period bounds, base currency, locale, colour scheme and
  the theme's custom properties). Whenever any of it changes the widget is rendered again with the
  new values. There is no request message, so there is nothing to validate and nothing to deny.
- **The data is a projection, not the wire.** Field names are the bridge's own and are versioned by
  the manifest's `api`, so an internal rename does not break a plugin. Money crosses as strings, as
  everywhere; instruments are named by ticker and name, never by an internal id.
- **The host owns the tile, the plugin owns the body.** Header, title, settings dialog, size and
  the plugin's name in the header are the host's (ADR-0082); the frame fills `.w__body`. A widget
  offers `title` and `source`, and `period` when its manifest says it is `periodic`.
- **Placing the tile is the consent.** What a widget reads is listed in Settings → Plugins and in
  the palette entry that adds it — the summary tile's rule (ADR-0039): the user is shown what it
  reads before the first time it does. It is a read of the tile's own scope and nothing leaves the
  machine, so no per-render `Ask` exists.
- **A failure is the tile's, not the board's.** The shim reports an uncaught error, a rejected
  promise and a module that never registered a renderer; the host shows that in the body with the
  plugin's own message. A frame that never says it is ready is reported the same way.
- **No fixture.** A drawing has no expected answer the host could compare, unlike a reader
  (ADR-0086). The install check is structural: the module exists and is `.js`, every read is known,
  the sizes fit the grid. This is the one kind that installs without one, and it is also the one
  kind that cannot write, reach the network or see anything but the reads it declared.
- **A widget whose plugin is gone stays on the board** and says so, like a theme preference
  naming a removed plugin (ADR-0070): removing it is the user's decision, not a side effect.

## Alternatives

- **`srcdoc` frames built by the frontend.** No host change, but the frame inherits the app's CSP,
  and loosening that to allow inline script weakens the app's own window to host a plugin.
- **A request/response bridge (`read("positions")`).** Lazier, and it turns every message into a
  permission check and every render into a round trip; declaring the reads up front gives the same
  data with no protocol to get wrong.
- **Handing over the internal wire types.** No mapping to maintain, and every rename in `lib/types`
  becomes a breaking change for somebody else's package.
- **A Web Worker or a WASM UI.** No DOM to draw into, or megabytes for a tile.

## Consequences

- The frame runs in the window's process: a widget stuck in a busy loop stalls the app, and nothing
  in a webview can interrupt it. Named, not solved — a sandboxed process per plugin is a desktop-only
  answer ADR-0070 already declined.
- Tauri's main-frame-only IPC is a property this design relies on. A Tauri upgrade that injected
  IPC into subframes would reopen it; the `connect-src`-less CSP still stands in front of the IPC
  scheme, but that is one wall instead of two.
- The bridge vocabulary grows by adding a read, never by changing one; changing one is a new `api`.
- Screens (8d) and assistant tools (8e) build on this bridge: a screen is a larger frame, a tool
  a body that receives the same projection.
