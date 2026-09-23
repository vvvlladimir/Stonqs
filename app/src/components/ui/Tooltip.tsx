import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

/** Delegated plain-text tooltips for elements marked with `data-tip`. */
interface Anchor {
  text: string;
  rect: DOMRect;
}

interface Placed {
  left: number;
  top: number;
  below: boolean;
}

/** Distance from the anchor, and the margin the bubble keeps to the window's edges. */
const GAP = 6;
const EDGE = 8;

export function TooltipLayer() {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const [placed, setPlaced] = useState<Placed | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const shown = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const clear = () => {
      shown.current = null;
      setAnchor(null);
    };
    const onOver = (e: Event) => {
      const target = (e.target as HTMLElement | null)?.closest?.("[data-tip]") as HTMLElement | null;
      const text = target?.getAttribute("data-tip");
      if (!target || !text) return clear();
      // Pointer moving inside the same element must not re-place the bubble under the cursor.
      if (shown.current === target) return;
      shown.current = target;
      setAnchor({ text, rect: target.getBoundingClientRect() });
    };

    document.addEventListener("pointerover", onOver);
    document.addEventListener("pointerleave", clear);
    document.addEventListener("pointerdown", clear);
    document.addEventListener("focusin", onOver);
    document.addEventListener("scroll", clear, true);
    // A bubble must be dismissible without moving the pointer or the focus (WCAG 1.4.13).
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAnchor(null);
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("pointerleave", clear);
      document.removeEventListener("pointerdown", clear);
      document.removeEventListener("focusin", onOver);
      document.removeEventListener("scroll", clear, true);
    };
  }, []);

  // Placed from the rendered size: a bubble is only as wide as its text, and the flip above/below
  // and the clamp to the window cannot be known before it is measured.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!anchor || !el) {
      setPlaced(null);
      return;
    }
    const { width, height } = el.getBoundingClientRect();
    const r = anchor.rect;
    const below = r.top - GAP - height < EDGE;
    const half = width / 2;
    const center = r.left + r.width / 2;
    setPlaced({
      left: Math.min(Math.max(center, EDGE + half), Math.max(EDGE + half, window.innerWidth - EDGE - half)),
      top: below ? r.bottom + GAP : r.top - GAP - height,
      below,
    });
  }, [anchor]);

  if (!anchor) return null;
  return createPortal(
    <div
      ref={ref}
      // A tip of several paragraphs is a list (the sync chip's failures) and gets room to read.
      className={anchor.text.includes("\n") ? "tip tip--long" : "tip"}
      role="tooltip"
      data-side={placed?.below ? "below" : "above"}
      style={placed ? { left: placed.left, top: placed.top, opacity: 1 } : { left: 0, top: 0 }}
    >
      {anchor.text}
    </div>,
    document.body,
  );
}
