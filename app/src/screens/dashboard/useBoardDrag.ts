import { useRef, useState, type RefObject } from "react";
import type { Widget } from "../../lib/uiState";
import { dropOrder, sameOrder, useEdgeScroll, useReflow, type TileBox } from "./grid";

/** The tile under the pointer: out of the flow, following the cursor from where it was grabbed. */
export interface Drag {
  id: string;
  box: DOMRect;
  dx: number;
  dy: number;
}

/**
 * Moving and resizing tiles. What the pointer does is kept out of the saved layout until the
 * gesture ends, so a drag writes settings once rather than on every move.
 */
export function useBoardDrag(
  widgets: Widget[],
  setBoard: (widgets: Widget[]) => void,
  gridRef: RefObject<HTMLDivElement | null>,
) {
  const [drag, setDrag] = useState<Drag | null>(null);
  const [order, setOrder] = useState<string[] | null>(null);
  // Where the pointer last was, so an edge scroll can re-ask the same question of a moved board.
  const point = useRef({ x: 0, y: 0 });
  const [preview, setPreview] = useState<({ id: string } & TileBox) | null>(null);

  const resize = (widget: Widget, box: TileBox, commit: boolean) => {
    if (!commit) {
      setPreview({ ...box, id: widget.id });
      return;
    }
    setPreview(null);
    const next = { ...widget, ...box };
    if (next.w === widget.w && next.h === widget.h && next.x === widget.x && next.y === widget.y) return;
    setBoard(widgets.map((w) => (w.id === widget.id ? next : w)));
  };

  /** What a tile is drawn at: the drag's preview while it lasts, the stored box otherwise. */
  const sized = (widget: Widget): Widget =>
    preview && preview.id === widget.id ? { ...widget, ...preview, id: widget.id } : widget;

  /** Widgets in the order they are drawn: the drag's own while it lasts, the stored one after. */
  const shown = order ? order.flatMap((id) => widgets.filter((w) => w.id === id)) : widgets;

  /** Re-asks where the dragged tile would land; on every move and every scroll step. */
  const dragOver = (id: string) => {
    const grid = gridRef.current;
    if (!grid) return;
    const next = dropOrder(grid, order ?? widgets.map((w) => w.id), id, point.current.x, point.current.y);
    setOrder((current) => (sameOrder(current, next) ? current : next));
  };

  // The board scrolls itself while a tile is held against the edge, and the board moved under a
  // pointer that did not, so the drag asks again after each step.
  const edge = useEdgeScroll(gridRef, () => {
    if (drag) dragOver(drag.id);
  });

  const start = (id: string, box: DOMRect) => setDrag({ id, box, dx: 0, dy: 0 });

  const move = (id: string, dx: number, dy: number, x: number, y: number) => {
    point.current = { x, y };
    setDrag((current) => (current ? { ...current, dx, dy } : current));
    edge.follow(y);
    dragOver(id);
  };

  const drop = (commit: boolean) => {
    edge.stop();
    setDrag(null);
    // A dropped tile lands where the flow puts it: a pinned place belonged to where it left.
    if (commit && order)
      setBoard(shown.map((w) => (w.id === drag?.id ? { ...w, x: undefined, y: undefined } : w)));
    setOrder(null);
  };

  // Only a move animates: a resize already follows the pointer.
  useReflow(
    gridRef,
    shown
      .map((w) => sized(w))
      .map((w) => `${w.id}:${w.w}x${w.h}@${w.x ?? ""},${w.y ?? 0}`)
      .join("|"),
    drag !== null,
  );

  return { drag, moving: drag !== null || preview !== null, shown, sized, resize, start, move, drop };
}
