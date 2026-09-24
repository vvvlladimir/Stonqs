import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

// The gesture itself is not the board's: the AI panel resizes with the same hook.
export { usePointerDrag, type DragGesture } from "../../lib/pointerDrag";
import type { WidgetDef } from "./widgets";
import type { Widget } from "../../lib/uiState";

/**
 * The dashboard grid.
 *
 * A widget stores one width, always in twelfths, and the board scales it to whatever the
 * screen affords. Storing a per-breakpoint layout instead would mean three layouts to keep in
 * step and three to export; scaling one is arithmetic, and the only thing that can be wrong
 * with it is rounding.
 */

/** Columns the stored width is expressed in. Twelve divides by 2, 3, 4 and 6. */
export const GRID_COLS = 12;

/**
 * Height of one row, in pixels, mirroring `--w-row` in `styles/ui/widget.css`. Small on
 * purpose: with the 12px gap a row costs 32px of board, so a tile is sized to its content
 * rather than to the nearest tall step.
 */
export const ROW_PX = 23;

/** Tallest a widget may be dragged: past this it is a screen, not a tile. */
export const MAX_ROWS = 40;

/**
 * Columns the board actually has at a width: a desktop gets all twelve, a tablet six, a phone
 * two. Width, not platform, for the same reason `useIsWide` measures it — a window shrunk to
 * 380px should lay out like a phone.
 */
const BREAKPOINTS: Array<[number, number]> = [
  [1000, 12],
  [700, 6],
];

export function columnsAt(width: number): number {
  return BREAKPOINTS.find(([at]) => width >= at)?.[1] ?? 2;
}

/**
 * The current column count, following the BOARD. Not the window: with the assistant panel open
 * the board is half the screen, and asking the window gave a 600px board twelve columns. The
 * stylesheet reads the same width through a container query, so the two never disagree.
 */
export function useBoardColumns(board: React.RefObject<HTMLElement | null>): number {
  // The window is the first guess — it is never narrower than the board — and the observer
  // corrects it on the first frame, before anything is painted twice.
  const [cols, setCols] = useState(() => columnsAt(window.innerWidth));

  useEffect(() => {
    const node = board.current;
    if (!node) return;
    const observer = new ResizeObserver(() => setCols(columnsAt(node.clientWidth)));
    observer.observe(node);
    return () => observer.disconnect();
  }, [board]);

  return cols;
}

/** The stored width scaled to a board of `cols` columns, never below one column. */
export function spanAt(w: number, cols: number): number {
  if (cols >= GRID_COLS) return clamp(w, 1, GRID_COLS);
  return clamp(Math.round((w * cols) / GRID_COLS), 1, cols);
}

/**
 * The smallest size this tile reads at: the catalog's, unless the board's owner named one.
 *
 * `cfg.min_w` is a width in twelfths like every other, and it is what decides whether the tile
 * takes a whole phone row (`shownSpan`) — so a list nobody wants shrunk and a chart that is
 * fine at a third of the board are the same setting, not a flag per widget.
 */
export function limitsOf(widget: Pick<Widget, "cfg">, def: WidgetDef): { w: number; h: number } {
  const own = Number(widget.cfg?.min_w);
  const w = Number.isInteger(own) && own >= 1 && own <= GRID_COLS ? own : def.min.w;
  return { w, h: def.min.h };
}

/** A size the catalog allows: never under the widget's own minimum, never over the board. */
export function fitSize(min: { w: number; h: number }, w: number, h: number): { w: number; h: number } {
  return {
    w: clamp(Math.round(w), min.w, GRID_COLS),
    h: clamp(Math.round(h), min.h, MAX_ROWS),
  };
}

/** What a resize changes: the size, plus the pinned start column and the empty rows above. */
export type TileBox = Pick<Widget, "w" | "h" | "x" | "y">;

/**
 * How wide a tile is actually drawn. The stored twelfths, except on a phone-width board: a
 * widget whose own minimum is a third of the board cannot live in half a phone column — every
 * chart declares `min.w >= 4`, and a plot 150px wide is a texture, not a reading. There it takes
 * the row, and what stays side by side is what was always small: the figures and the lists.
 *
 * The rule is read off that minimum (`limitsOf`), so no widget gains a "wide on mobile" flag.
 */
export function shownSpan(box: TileBox, cols: number, minW: number): number {
  if (cols <= 2 && minW >= 4) return cols;
  return spanAt(box.w, cols);
}

/**
 * Where a widget sits on a board of `cols` columns. The flow still places it — `x` only says
 * which column it starts in (a start the flow has already passed wraps it to the next row), and
 * `y` rows of its own slot are left empty above it, which is how a bottom edge stays put.
 */
export function placement(box: TileBox, cols: number, minW: number): React.CSSProperties {
  const span = shownSpan(box, cols, minW);
  const top = box.y ?? 0;
  const start = box.x === undefined ? null : clamp(scaleDown(box.x, cols), 0, cols - span);
  return {
    // eslint-disable-next-line lingui/no-unlocalized-strings -- a grid line, not a sentence
    gridColumn: start === null ? `span ${span}` : `${start + 1} / span ${span}`,
    gridRow: `span ${box.h + top}`,
    // eslint-disable-next-line lingui/no-unlocalized-strings -- a CSS length, not a sentence
    marginTop: top > 0 ? `calc(${top} * (var(--w-row) + var(--w-gap)))` : undefined,
  };
}

/** Twelfths onto a board of `cols` columns, and back. */
function scaleDown(twelfths: number, cols: number): number {
  return cols >= GRID_COLS ? twelfths : Math.round((twelfths * cols) / GRID_COLS);
}
function scaleUp(shown: number, cols: number): number {
  return cols >= GRID_COLS ? shown : Math.round((shown * GRID_COLS) / cols);
}

/** One column and one row of the live board, gap included, measured rather than assumed. */
export function cellSize(grid: HTMLElement): { width: number; height: number } {
  const style = getComputedStyle(grid);
  const gap = parseFloat(style.columnGap) || 0;
  const cols = columnsAt(grid.clientWidth);
  const row = parseFloat(style.getPropertyValue("--w-row")) || ROW_PX;
  return { width: (grid.clientWidth - gap * (cols - 1)) / cols + gap, height: row + gap };
}

/** A tile's slot: its visible box plus the empty rows it keeps above itself. */
function slotOf(el: HTMLElement): DOMRect {
  const box = el.getBoundingClientRect();
  const above = parseFloat(getComputedStyle(el).marginTop) || 0;
  return new DOMRect(box.left, box.top - above, box.width, box.height + above);
}

/**
 * Where a tile starts, in the board's columns, and the leftmost column its left edge may be
 * dragged to: the right edge of the nearest tile beside it in any row it spans. Past that the
 * flow would wrap it to the next row, under a pointer that asked for no such thing.
 */
export function columnsOf(grid: HTMLElement, tile: HTMLElement): { start: number; free: number } {
  const pitch = cellSize(grid).width;
  const origin = grid.getBoundingClientRect().left;
  const own = slotOf(tile);
  const col = (px: number) => Math.round((px - origin) / pitch);
  const free = Array.from(grid.querySelectorAll<HTMLElement>("[data-id]"))
    .filter((el) => el !== tile)
    .map(slotOf)
    .filter((box) => box.top < own.bottom - 1 && box.bottom > own.top + 1 && box.right <= own.left + 1)
    .reduce((edge, box) => Math.max(edge, col(box.right)), 0);
  return { start: col(own.left), free: Math.min(free, col(own.left)) };
}

/** Which edges a handle moves: `-1` is the left or top one, `0` leaves that axis alone. */
export interface GrowDirection {
  x: -1 | 0 | 1;
  y: -1 | 0 | 1;
}

/**
 * The box a drag of `dx`/`dy` pixels asks for. The edge under the pointer is the one that
 * moves and the opposite one stays: dragging the left edge re-pins the start column, dragging
 * the top one trades rows between the tile and the empty space above it — which runs out at
 * the top of the tile's slot, because the flow has no rows to give it above that.
 */
export function draggedBox(
  grid: HTMLElement,
  min: { w: number; h: number },
  start: TileBox,
  at: { start: number; free: number },
  dir: GrowDirection,
  dx: number,
  dy: number,
): TileBox {
  const cell = cellSize(grid);
  const cols = columnsAt(grid.clientWidth);
  const stepX = Math.round(dx / cell.width);
  const stepY = Math.round(dy / cell.height);
  const span = spanAt(start.w, cols);
  const top = start.y ?? 0;
  const next: TileBox = { ...start };

  // The drag moves the *rendered* width; convert back so a tablet drag of one column is not
  // recorded as one twelfth of the board.
  if (dir.x === 1) {
    next.w = fitSize(min, scaleUp(clamp(span + stepX, 1, cols), cols), start.h).w;
  } else if (dir.x === -1) {
    const end = at.start + span;
    next.w = fitSize(min, scaleUp(clamp(span - stepX, 1, end - at.free), cols), start.h).w;
    next.x = scaleUp(Math.max(end - spanAt(next.w, cols), 0), cols);
  }

  if (dir.y === 1) {
    next.h = fitSize(min, start.w, start.h + stepY).h;
  } else if (dir.y === -1) {
    const bottom = top + start.h;
    next.h = Math.min(fitSize(min, start.w, start.h - stepY).h, bottom);
    next.y = bottom - next.h;
  }
  return next;
}

/** The eight handles of a tile. The edge a handle sits on is the one it moves. */
export const HANDLES: Array<{ key: string; dir: GrowDirection }> = [
  { key: "n", dir: { x: 0, y: -1 } },
  { key: "s", dir: { x: 0, y: 1 } },
  { key: "w", dir: { x: -1, y: 0 } },
  { key: "e", dir: { x: 1, y: 0 } },
  { key: "nw", dir: { x: -1, y: -1 } },
  { key: "ne", dir: { x: 1, y: -1 } },
  { key: "sw", dir: { x: -1, y: 1 } },
  { key: "se", dir: { x: 1, y: 1 } },
];

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}

/** Every tile the board currently draws, in draw order, as boxes in client coordinates. */
function tileBoxes(grid: HTMLElement, skip: string): Array<{ id: string; box: DOMRect }> {
  return Array.from(grid.querySelectorAll<HTMLElement>("[data-id]"))
    .map((el) => ({ id: el.dataset.id ?? "", box: el.getBoundingClientRect() }))
    .filter(({ id }) => id !== "" && id !== skip);
}

/**
 * The order the pointer is asking for: `moved` is lifted out and put back where the pointer
 * points. The landing place is read off the boxes the board has actually laid out — an
 * insertion point, not a swap with whatever tile is under the cursor. A swap oscillates the
 * moment the two tiles are different sizes, because the taller one lands back under the
 * pointer and asks to be swapped again.
 */
export function dropOrder(grid: HTMLElement, ids: string[], moved: string, x: number, y: number): string[] {
  const rest = ids.filter((id) => id !== moved);
  const boxes = tileBoxes(grid, moved);
  // Reading order: before the first tile that starts below the pointer, or, inside the row the
  // pointer is in, before the first tile whose middle it has not passed yet.
  const hit = boxes.findIndex(({ box }) => y < box.top || (y <= box.bottom && x < box.left + box.width / 2));
  const at = hit < 0 ? rest.length : rest.indexOf(boxes[hit].id);
  const next = [...rest];
  next.splice(at < 0 ? rest.length : at, 0, moved);
  return next;
}

export function sameOrder(a: string[] | null, b: string[]): boolean {
  return a !== null && a.length === b.length && a.every((id, i) => id === b[i]);
}

/**
 * Animates each tile from where it was to where the new order puts it (FLIP): the grid places
 * tiles in one frame, so a CSS transition has nothing to interpolate. Positions are kept
 * relative to the board so that scrolling — which the drag does on its own at the edges — is
 * not mistaken for a tile that moved.
 */
export function useReflow(grid: React.RefObject<HTMLElement | null>, layout: string, animate: boolean) {
  const before = useRef(new Map<string, { x: number; y: number }>());

  useLayoutEffect(() => {
    const node = grid.current;
    if (!node) return;
    const origin = node.getBoundingClientRect();
    const now = new Map<string, { x: number; y: number }>();
    const moves: Array<[HTMLElement, number, number]> = [];

    node.querySelectorAll<HTMLElement>("[data-id]").forEach((el) => {
      const id = el.dataset.id;
      // The tile under the pointer is placed by the pointer; only the board's own tiles move.
      if (!id || el.classList.contains("is-drag")) return;
      const box = el.getBoundingClientRect();
      const at = { x: box.left - origin.left, y: box.top - origin.top };
      const was = before.current.get(id);
      now.set(id, at);
      if (was && (was.x !== at.x || was.y !== at.y)) moves.push([el, was.x - at.x, was.y - at.y]);
    });

    before.current = now;
    if (!animate || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    for (const [el, dx, dy] of moves) {
      // eslint-disable-next-line lingui/no-unlocalized-strings -- a CSS transform, not a sentence
      const from = `translate(${dx}px, ${dy}px)`;
      el.animate([{ transform: from }, { transform: "none" }], {
        duration: 180,
        easing: "cubic-bezier(0.2, 0, 0, 1)",
      });
    }
  }, [grid, layout, animate]);
}

/** How close to the scroller's edge the pointer has to get, and the fastest it then scrolls. */
const EDGE_PX = 64;
const EDGE_SPEED = 16;

/** The nearest ancestor that scrolls; the board itself never does. */
function scrollerOf(node: HTMLElement): HTMLElement {
  for (let el = node.parentElement; el; el = el.parentElement) {
    const overflow = getComputedStyle(el).overflowY;
    if ((overflow === "auto" || overflow === "scroll") && el.scrollHeight > el.clientHeight) return el;
  }
  return document.scrollingElement as HTMLElement;
}

/**
 * Scrolls the board while the pointer is held against the top or bottom of its scroller, so a
 * tile can reach the far end of a long board. A pointer resting there still scrolls — hence a
 * frame loop rather than a nudge per pointer move — and `onScroll` lets the drag re-read where
 * it would now land.
 */
export function useEdgeScroll(grid: React.RefObject<HTMLElement | null>, onScroll: () => void) {
  const notify = useRef(onScroll);
  useLayoutEffect(() => {
    notify.current = onScroll;
  });

  const scroll = useMemo(() => {
    let frame = 0;
    let at: number | null = null;

    function step() {
      frame = 0;
      const node = grid.current;
      if (at === null || !node) return;
      const box = scrollerOf(node);
      const view =
        box === document.scrollingElement
          ? new DOMRect(0, 0, window.innerWidth, window.innerHeight)
          : box.getBoundingClientRect();
      // Positive above the top edge, negative below the bottom one, zero in between.
      const over = Math.max(0, view.top + EDGE_PX - at) || -Math.max(0, at - (view.bottom - EDGE_PX));
      if (over !== 0) {
        box.scrollTop -= Math.round((over / EDGE_PX) * EDGE_SPEED);
        notify.current();
      }
      frame = window.requestAnimationFrame(step);
    }

    return {
      follow(y: number) {
        at = y;
        if (!frame) frame = window.requestAnimationFrame(step);
      },
      stop() {
        at = null;
        if (frame) window.cancelAnimationFrame(frame);
        frame = 0;
      },
    };
  }, [grid]);

  useEffect(() => scroll.stop, [scroll]);
  return scroll;
}
