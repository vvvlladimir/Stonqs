import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";

/**
 * Reordering inside the navigation, and pinning by dropping on Favorites.
 *
 * Window listeners, not HTML5 drag and drop (absent under a finger) and not pointer capture (the
 * list re-renders under the pointer) — the same reasons as `lib/pointerDrag`. That hook starts a
 * drag on any press; here a touch must be *held* first, or the sheet could never be scrolled.
 * The target is found by coordinates over marked rows: `data-nav-section`, `data-nav-fav`,
 * `data-nav-screen`, each carrying `data-id` and `data-key`, inside `data-nav-favorites` and
 * `data-nav-sub` lists.
 */
export type DragKind = "fav" | "screen" | "section";

export interface DragItem {
  kind: DragKind;
  id: string;
  /** The section a screen row is moved within; a screen never leaves its section. */
  section?: string;
}

export type Drop = { kind: "section" | "screen" | "fav"; before: string | null } | { kind: "unfav" };

/** Where the insertion line goes: above the row `key`, or below it. */
export interface Mark {
  key: string;
  side: "before" | "after";
}

export interface DragState {
  item: DragItem;
  /** Top-left of the ghost, so it stays where the row was grabbed. */
  x: number;
  y: number;
  width: number;
  drop: Drop | null;
  mark: Mark | null;
  overFavorites: boolean;
}

const SLOP = 5;
const HOLD_MS = 350;
/** A list is still the target this far outside its box, so its first and last gap are reachable. */
const REACH = 8;

type Hit = Pick<DragState, "drop" | "mark" | "overFavorites">;

export function useReorder(
  root: RefObject<HTMLElement | null>,
  onDrop: (item: DragItem, drop: Drop) => void,
) {
  const [drag, setDrag] = useState<DragState | null>(null);
  const live = useRef(onDrop);
  const stop = useRef<() => void>(() => {});

  useLayoutEffect(() => {
    live.current = onDrop;
  });
  useEffect(() => () => stop.current(), []);

  const start = (item: DragItem) => (e: React.PointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    stop.current();
    const el = e.currentTarget;
    const x0 = e.clientX;
    const y0 = e.clientY;
    const touch = e.pointerType !== "mouse";
    let armed = !touch;
    let on = false;
    let grab = { x: 0, y: 0, width: 0 };
    let last: DragState | null = null;
    const hold = touch ? window.setTimeout(() => (armed = true), HOLD_MS) : 0;

    const move = (ev: PointerEvent) => {
      if (!on) {
        if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < SLOP) return;
        // A finger that moved before the hold is scrolling the list, not carrying a row.
        if (!armed) return finish(false);
        on = true;
        const r = el.getBoundingClientRect();
        grab = { x: x0 - r.left, y: y0 - r.top, width: r.width };
      }
      last = {
        item,
        x: ev.clientX - grab.x,
        y: ev.clientY - grab.y,
        width: grab.width,
        ...locate(root.current, el, item, ev.clientX, ev.clientY),
      };
      setDrag(last);
    };
    const finish = (commit: boolean) => {
      stop.current();
      if (!on) return;
      setDrag(null);
      if (commit && last?.drop) live.current(item, last.drop);
      // The release after a drag is not a press of the row it landed on.
      const swallow = (ev: MouseEvent) => {
        ev.stopPropagation();
        ev.preventDefault();
      };
      window.addEventListener("click", swallow, true);
      window.setTimeout(() => window.removeEventListener("click", swallow, true), 0);
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    // A keyboard name, not text; an object key keeps it out of the catalogue.
    const keys = (ev: KeyboardEvent) => {
      if ({ Escape: true }[ev.key as "Escape"]) cancel();
    };
    // Once a held touch is armed, the page must not scroll under the row it carries.
    const pin = (ev: TouchEvent) => {
      if (armed) ev.preventDefault();
    };
    const menu = (ev: Event) => {
      if (touch) ev.preventDefault();
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", keys);
    window.addEventListener("touchmove", pin, { passive: false });
    window.addEventListener("contextmenu", menu);
    stop.current = () => {
      window.clearTimeout(hold);
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", keys);
      window.removeEventListener("touchmove", pin);
      window.removeEventListener("contextmenu", menu);
      stop.current = () => {};
    };
  };

  return { drag, start };
}

function locate(root: HTMLElement | null, el: HTMLElement, item: DragItem, x: number, y: number): Hit {
  const none: Hit = { drop: null, mark: null, overFavorites: false };
  if (!root) return none;
  const rows = (scope: ParentNode, selector: string) =>
    [...scope.querySelectorAll<HTMLElement>(selector)].filter((row) => row !== el);

  if (item.kind === "section") {
    const heads = rows(root, "[data-nav-section]");
    const before = beforeOf(heads, y);
    // Past the last head the line goes under the last section's list, not between it and its head.
    const end = root.querySelector<HTMLElement>("[data-nav-end]");
    const mark = before ?? end;
    return {
      drop: { kind: "section", before: idOf(before) },
      mark: mark ? { key: mark.dataset.key ?? "", side: "before" } : null,
      overFavorites: false,
    };
  }

  // Favorites take a screen from any section as well as their own rows.
  const favorites = root.querySelector<HTMLElement>("[data-nav-favorites]");
  if (favorites && inside(favorites, x, y)) {
    const list = rows(favorites, "[data-nav-fav]");
    const before = beforeOf(list, y);
    // An empty list still has Overview to draw the line under.
    const floor = list.length ? list : rows(favorites, "[data-key]");
    return { drop: { kind: "fav", before: idOf(before) }, mark: markOf(before, floor), overFavorites: true };
  }
  if (item.kind === "fav") return { drop: { kind: "unfav" }, mark: null, overFavorites: false };

  const sub = el.closest<HTMLElement>("[data-nav-sub]");
  if (sub && inside(sub, x, y)) {
    const list = rows(sub, "[data-nav-screen]");
    const before = beforeOf(list, y);
    return {
      drop: { kind: "screen", before: idOf(before) },
      mark: markOf(before, list),
      overFavorites: false,
    };
  }
  return none;
}

/** The first row whose middle is below the pointer; `null` is the end of the list. */
function beforeOf(rows: HTMLElement[], y: number): HTMLElement | null {
  return (
    rows.find((row) => {
      const r = row.getBoundingClientRect();
      return y < r.top + r.height / 2;
    }) ?? null
  );
}

function markOf(before: HTMLElement | null, rows: HTMLElement[]): Mark | null {
  if (before) return { key: before.dataset.key ?? "", side: "before" };
  const lastRow = rows[rows.length - 1];
  return lastRow ? { key: lastRow.dataset.key ?? "", side: "after" } : null;
}

function idOf(row: HTMLElement | null): string | null {
  return row?.dataset.id ?? null;
}

function inside(el: HTMLElement, x: number, y: number): boolean {
  const r = el.getBoundingClientRect();
  return x >= r.left - REACH && x <= r.right + REACH && y >= r.top - REACH && y <= r.bottom + REACH;
}
