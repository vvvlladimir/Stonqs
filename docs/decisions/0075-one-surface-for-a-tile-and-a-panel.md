# 75: One surface for a dashboard tile and a screen's panel

- Status: Accepted

## Context

A dashboard tile (`.w`) and a screen's panel (`.panel`) are the same object: a bordered card on a
neutral page holding a heading and one block. They were written twice and had drifted apart — the
panel wore a 10px radius and 16px of padding, the tile a 16px radius and 12px — so the same chart,
the same allocation bar and the same table read as two different drawings depending on which
screen they were on.

The drift also hid a dead rule. `styles/ui/widget.css` set the tile's padding from
`@container w (max-width: 200px) { .w { --w-pad: … } }`. An element cannot answer a container
query it declares itself: that selector only ever matches a `.w` nested inside another `.w`, which
never happens. The tile's density steps had never once applied, and the grab handle's negative
margins hardcoded the value they were supposed to follow.

ADR-0073 made every tile a container so that what it holds is laid out by the box it is in. A
panel was left out of that, so the same block asked the window on a screen and the tile on a board.

## Decision

One class, `.box` (`styles/ui/surface.css`), carries what a top-level surface is: the background,
the border, the large radius, `--box-pad` and its padding, `min-width: 0`, and
`container-type: inline-size`. `.w` and `.panel` are both `.box` plus what each adds — the tile
overrides the containment to `size`, because its height is a row span and a block may ask how
short it is; a panel is as tall as what it holds.

A box that has to be tighter names a value, not a padding shorthand: `.panel--table` reaches its
own edges with `--box-pad: var(--sp-3)`, a tile on a phone board tightens to `var(--sp-5)`. That
last rule is asked of the **board** container, which the tile is a descendant of, rather than of
the tile itself.

The page's other own cards follow the same radius: the figure shelf, the empty state, a banner and
the violations box. Anything *inside* a box keeps `--r`/`--r-input` — one shape scale, three steps.

## Alternatives

- **Leave two surfaces and keep them in step by hand.** They were not in step; that is the
  context.
- **Move the padding onto `.box__head`/`.box__body` so a box can still answer its own width.**
  Correct in principle and a much larger change: every bare `.panel` — the gate's card, the edit
  bar — would silently lose its padding, and the tile's grab handle, its scroll fade and its
  masks all sit on the box's own padding today.
- **Give the panel `container-type: size`.** A panel has no height of its own; size containment
  would collapse it.

## Consequences

- A panel is now a query container, so a block inside one reshapes by the panel and not by the
  window. Blocks that declare their own container (`.alloc`, `.drift`, `.wfall`, `.calbox`) are
  unaffected; `.fig`/`.prog`, which do not, now answer to the panel on a screen instead of to the
  viewport.
- `contain: layout` comes with `container-type`, so a panel is a containing block for fixed
  descendants. Every overlay in the app is portaled to `body`, so none is affected — a new one
  must be too.
- Tiles are 16px-padded on a tablet or desktop board and 12px on a phone one, which is what the
  dead rules intended and never did.
- `app/mockups/` renders the real stylesheets, so the shared surface is checked there at any board
  width.
