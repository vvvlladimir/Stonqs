# 29: A dashboard widget stores one size in twelfths

- Status: Accepted; resizing from the top and left edges superseded by ADR-0045

## Context

The dashboard was a flow of tiles whose only size was `span`, a width in quarters of a
four-column board. The board itself had three shapes — one column on a phone, two above 480px,
four above 1000px — and `span` was mapped onto them by CSS, so anything wider than a half
collapsed to a half on a tablet. Height was whatever the content came to, which made two tiles
in one row rarely line up.

Three things were wanted at once:

- a widget should have a size the catalog considers right, and the user should be able to
  change it by dragging the tile's edge rather than stepping through a list of allowed widths;
- the same gesture has to work under a finger, because tablets and phones are target platforms;
- a layout has to leave the app as a file — so a curated board can ship with the product, and
  so a user can keep or share their own.

A per-breakpoint layout (the react-grid-layout model: one placement per breakpoint, free `x`/`y`
with collision resolution) answers the first two and makes the third worse: three layouts to
keep in step, three to export, and a file that is wrong on a screen it was not authored for.

## Decision

A widget stores **one** width `w`, always in twelfths of the board, and one height `h` in grid
rows. The board renders 12 columns on a desktop, 6 on a tablet and 2 on a phone, and scales the
stored width onto whatever it has: `round(w * cols / 12)`, never below one column. Placement
stays a flow — order plus size, no coordinates — so there is nothing to collide and nothing to
repack when the column count changes.

`WidgetDef` gains `size` (what a freshly added widget gets) and `min` (where dragging stops).
Both are the catalog's opinion, not the user's, and neither is stored per widget.

A row is deliberately short — 20px, so with the 12px gap one row costs 32px of board. A tile is
then sized to what is in it rather than to the nearest tall step, and a divider is a line rather
than a box.

Both gestures are pointer events, so mouse and touch are one code path; HTML5 drag and drop is
not used anywhere, because it does not exist under a finger.

- **Resizing** happens on handles at every edge and every corner. The top and left ones grow the
  tile too: the board is a flow, so there is no far edge to pin — what the pointer drags is the
  size, not a rectangle's coordinates. While the pointer is down the size is a preview held by
  the screen; it reaches settings once, on release. The bottom-right handle is a button and
  answers the arrow keys, because a keyboard has no drag.
- **Moving** happens on the tile's header, which is the handle — a press inside the widget's
  body still belongs to the widget, and a press on a tool is a click on that tool. The press
  becomes a drag only after 6px of travel, so a tap is still a tap. The tile then leaves the
  flow and follows the pointer, and a placeholder keeps its slot: the placeholder is what
  reorders, so the board shows where the widget would land while it is still being carried.
  Escape puts it back. The landing place is an *insertion point* read off the boxes the board
  has laid out (`dropOrder`) — before the first tile that starts below the pointer, or, within
  a row, before the first tile whose middle it has not passed. Swapping with whatever tile sits
  under the cursor was the first attempt and oscillates the moment the two tiles are different
  sizes: the taller one lands back under the pointer and asks to be swapped again. Tiles that
  change place animate from where they were (FLIP, `useReflow`), because a grid places its
  items in one frame and a CSS transition has nothing to interpolate. A tile held against the
  edge of the scroller scrolls the board (`useEdgeScroll`), so a long board is reachable.

Both gestures listen on `window`, not on the pressed element (`usePointerDrag`). Pointer
capture is the obvious way to keep the moves coming and was what this did first, but moving a
tile reorders the board, React moves the captured node with `insertBefore`, and a browser reads
that as the element leaving the document and drops the capture — the gesture died the first
time the board reordered, which is to say the first time a big tile (a chart) was crossed.
While a gesture runs the board also stops hit-testing widget bodies, so a chart does not chase
a pointer that is not about it.

Neither gesture may leave a trail of selected text: the header's `pointerdown` is prevented, the
handles and the header are `user-select: none`, and the board wears `is-moving` for as long as a
gesture runs, which turns selection off across every tile the pointer crosses.

The stored blob carries `version` (`UI_VERSION`, currently 3). `parseUiState` migrates on read:
a widget with `span` and no `w` becomes `w = span * 3` and takes a height from a small fallback
table; a height written before version 3 counts three of today's rows, because that version cut
the row to a third of its height.

A board is a file: `{ kind: "stonqs.dashboard", version, dashboard }`, written and read by
`boardToFile` / `boardFromFile`, which go through the same migration — an older file still
opens. `lib/defaultDashboard.json` is a bare board in that format, so the dashboard that ships
is authored by exporting one from the app rather than by editing code.

## Alternatives

- **Keep `span` and add `rows`.** Cheapest, but leaves the quarter-board grid: on a 12-column
  desktop a third of the width is not expressible, and the tablet keeps collapsing halves.
- **A layout per breakpoint.** What most grid libraries do. Rejected above: it multiplies the
  thing that has to be exported, and the export is a requirement here, not a nicety.
- **Free `x`/`y` placement.** Needs collision resolution and compaction, and every one of those
  rules has to be reapplied when a phone drops the board to two columns — the result is a
  layout the user did not arrange on either screen.
- **A host command for reading and writing the file.** The host stores the blob opaquely and a
  webview file picker already works on every target, so a Rust counterpart would add a boundary
  crossing without adding a capability.

## Consequences

- Rows are a fixed height (`--w-row`), so a tile that is too short scrolls its own body rather
  than pushing the grid out of alignment. Sizes in the catalog have to be chosen with that in
  mind; a widget added at too small a default reads as a scrollbar.
- Changing `--w-row` changes what every stored height means, so it costs a `UI_VERSION` bump and
  a line in the migration. The constant is mirrored in `grid.ts` as `ROW_PX`, read from the live
  board where one is available and used as the fallback where one is not.
- Scaling rounds, so on a 6-column tablet a width of 5/12 and one of 6/12 both render as 3
  columns. That is accepted: the alternative is storing what the tablet should do separately.
- The width picker is gone from the configuration dialog — the tile's edge is the one way to
  resize, and a second control would be a second source of truth.
- The shipped board carries section headings, and their text is user data seeded in English —
  the same rule the default taxonomies follow (migration `0010`): the user renames them, and the
  UI never translates them back.
- `FALLBACK_SIZE` in `lib/uiState.ts` duplicates two entries of the catalog. The layout module
  must not import the dashboard screen, and migration has to work for a type the catalog no
  longer has; this is the one place the two are allowed to know the same thing.
