import { useEffect, useLayoutEffect, useRef } from "react";

/**
 * A press that becomes a drag, on `window` rather than on the pressed element.
 *
 * Pointer capture would be the obvious way to keep the moves coming, and it is what this used
 * to do — but moving a tile reorders the board, React moves the captured node with
 * `insertBefore`, and a browser reads that as the element leaving the document and drops the
 * capture. The gesture then died the first time the board reordered, which is exactly when a
 * big tile (a chart) was crossed. Window listeners have no such thing to lose.
 */
export interface DragGesture {
  /** Pixels the pointer must travel before the press counts as a drag. Zero starts at once. */
  slop?: number;
  onStart?: () => void;
  onMove: (dx: number, dy: number, x: number, y: number) => void;
  /** `commit` is false when the gesture was cancelled (Escape, or the pointer taken away). */
  onEnd: (commit: boolean) => void;
}

export function usePointerDrag(gesture: DragGesture): (e: React.PointerEvent) => void {
  // Read through a ref: the listeners outlive the render that installed them.
  const live = useRef(gesture);
  const from = useRef<{ x: number; y: number; started: boolean } | null>(null);
  const detach = useRef<() => void>(() => {});
  const abandon = useRef<() => void>(() => {});

  // Layout, not passive: the next pointer move must see the order the DOM was just given.
  useLayoutEffect(() => {
    live.current = gesture;
  });
  useEffect(() => () => detach.current(), []);

  return (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    // Without this the press starts a text selection and the drag paints the board blue.
    e.preventDefault();
    e.stopPropagation();
    // A gesture still open is one whose release never reached the window (let go outside it,
    // or swallowed by a native drag). Returning here instead left this element dead for good.
    if (from.current) abandon.current();
    const slop = live.current.slop ?? 0;
    from.current = { x: e.clientX, y: e.clientY, started: slop === 0 };
    if (from.current.started) live.current.onStart?.();

    const move = (event: PointerEvent) => {
      const start = from.current;
      if (!start) return;
      // The button is up but no `pointerup` came: end the gesture where the pointer left it.
      if (event.buttons === 0) return finish(true);
      const dx = event.clientX - start.x;
      const dy = event.clientY - start.y;
      if (!start.started) {
        if (Math.abs(dx) + Math.abs(dy) < slop) return;
        start.started = true;
        live.current.onStart?.();
      }
      live.current.onMove(dx, dy, event.clientX, event.clientY);
    };
    const finish = (commit: boolean) => {
      const start = from.current;
      from.current = null;
      detach.current();
      detach.current = () => {};
      if (start?.started) live.current.onEnd(commit);
    };
    const up = () => finish(true);
    const cancel = () => finish(false);
    abandon.current = cancel;
    // A keyboard name, not text; an object key keeps it out of the catalogue.
    const keys = (event: KeyboardEvent) => {
      if ({ Escape: true }[event.key as "Escape"]) cancel();
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", keys);
    detach.current = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", keys);
    };
  };
}
