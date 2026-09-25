import { useEffect, useState } from "react";

/** How long a step waits for its element before deciding the screen simply does not have it. */
const TIMEOUT = 2000;

export interface AnchorState {
  rect: DOMRect | null;
  /** The element never appeared: the step is about something this screen is not showing. */
  missing: boolean;
}

/**
 * Follows the element a step points at, across the screen change that precedes it.
 *
 * A screen arrives lazily and its data a moment later, so the element is polled rather than
 * looked up once — and polling continues after it is found, because the box moves when the
 * window is scrolled or resized. An element that never arrives is not an error: the step is
 * skipped by whoever asked (`missing`), so a deleted widget or a dock control that a narrow
 * window does not draw cannot stall the tour.
 */
const NONE: AnchorState = { rect: null, missing: false };

export function useAnchor(name: string | undefined, stepId: string): AnchorState {
  const [state, setState] = useState<AnchorState>(NONE);

  useEffect(() => {
    if (!name) return;
    let frame = 0;
    let scrolled = false;
    const started = performance.now();

    const tick = () => {
      const el = document.querySelector(`[data-tour="${name}"]`);
      if (el) {
        if (!scrolled) {
          scrolled = true;
          el.scrollIntoView({ block: "center", behavior: "smooth" });
        }
        const rect = el.getBoundingClientRect();
        // Only a moved box is a re-render: this runs every frame for as long as the step is up.
        setState((current) => (same(current.rect, rect) ? current : { rect, missing: false }));
      } else if (performance.now() - started > TIMEOUT) {
        setState({ rect: null, missing: true });
        return;
      }
      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
    // `stepId` restarts the wait when one step follows another on the same anchor name.
  }, [name, stepId]);

  // A step with no anchor is about the screen as a whole; the card is centred and nothing is
  // cut out. The state is per step anyway — the card is keyed by it and mounts afresh.
  return name ? state : NONE;
}

function same(a: DOMRect | null, b: DOMRect): boolean {
  return a !== null && a.top === b.top && a.left === b.left && a.width === b.width && a.height === b.height;
}
