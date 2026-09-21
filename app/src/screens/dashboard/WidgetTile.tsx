import { useRef } from "react";
import { useLingui } from "@lingui/react/macro";
import { DotsSixVerticalIcon, GearIcon, TrashIcon } from "@phosphor-icons/react";
import type { Widget } from "../../lib/uiState";
import { usePeriodRanges, type PeriodId } from "../../lib/periods";
import {
  columnsOf,
  draggedBox,
  fitSize,
  HANDLES,
  placement,
  usePointerDrag,
  type GrowDirection,
  type TileBox,
} from "./grid";
import { WIDGETS, widgetMeta, widgetTitle } from "./widgets";

/** Pixels a pointer must travel before a press on the header becomes a drag. */
const SLOP = 6;

/** Where a tile being moved is drawn: its own box at the press, plus how far the pointer went. */
export interface FloatAt {
  x: number;
  y: number;
  w: number;
  h: number;
  dx: number;
  dy: number;
}

/** One board cell: the widget's own render plus the edit-mode chrome around it. */
export function WidgetTile({
  widget,
  date,
  period,
  editing,
  float,
  cols,
  gridRef,
  onMoveStart,
  onMove,
  onMoveEnd,
  onResize,
  onConfig,
  onRemove,
}: {
  widget: Widget;
  date: string;
  period: PeriodId;
  editing: boolean;
  /** Set while this tile is the one under the pointer; it then follows it, out of the flow. */
  float: FloatAt | null;
  /** Columns the board has right now; the stored width is scaled onto them. */
  cols: number;
  gridRef: React.RefObject<HTMLDivElement | null>;
  /** The tile's box at the moment of the press — what the floating tile is drawn from. */
  onMoveStart: (box: DOMRect) => void;
  /** How far the pointer has travelled, and where it is now in client coordinates. */
  onMove: (dx: number, dy: number, x: number, y: number) => void;
  /** `commit` is false when the gesture was cancelled rather than released. */
  onMoveEnd: (commit: boolean) => void;
  /** `commit` is false while the pointer is still down: a preview, not a saved layout. */
  onResize: (box: TileBox, commit: boolean) => void;
  onConfig: () => void;
  onRemove: () => void;
}) {
  const { t, i18n } = useLingui();
  // Cached beside every other caller of the same date, so naming the period costs no query.
  const ranges = usePeriodRanges(date);
  const tile = useRef<HTMLElement | null>(null);
  const def = WIDGETS[widget.type];

  // One gesture for all eight handles: which one was pressed is the only thing that differs,
  // and a hook cannot be called per handle inside the map below.
  const grab = useRef<{
    dir: GrowDirection;
    from: TileBox;
    at: { start: number; free: number };
    box: TileBox;
  }>({ dir: { x: 0, y: 0 }, from: widget, at: { start: 0, free: 0 }, box: widget });

  const startResize = usePointerDrag({
    onStart: () => {
      const from = { w: widget.w, h: widget.h, x: widget.x, y: widget.y };
      // Measured once, at the press: the columns beside the tile are what the flow laid out
      // before the preview started moving it.
      const at =
        gridRef.current && tile.current ? columnsOf(gridRef.current, tile.current) : { start: 0, free: 0 };
      grab.current = { ...grab.current, from, at, box: from };
    },
    onMove: (dx, dy) => {
      if (!gridRef.current || !def) return;
      const { from, at, dir } = grab.current;
      grab.current.box = draggedBox(gridRef.current, def, from, at, dir, dx, dy);
      onResize(grab.current.box, false);
    },
    // A cancelled resize commits the box the tile started at, which drops the preview.
    onEnd: (commit) => onResize(commit ? grab.current.box : grab.current.from, true),
  });

  /**
   * Moving a tile is the same gesture as resizing one, for the same reason: HTML5 drag and drop
   * does not exist under a finger. The header is the handle, so a press inside the widget's own
   * body still belongs to the widget.
   */
  const startMove = usePointerDrag({
    slop: SLOP,
    onStart: () => tile.current && onMoveStart(tile.current.getBoundingClientRect()),
    onMove,
    onEnd: onMoveEnd,
  });

  if (!def) return null;
  const meta = widgetMeta(i18n, widget, def, period, ranges.data);

  return (
    <section
      ref={tile}
      className={`w${def.plain ? " w--plain" : ""}${float ? " is-drag" : ""}`}
      data-id={widget.id}
      style={
        float
          ? // Out of the flow and under the pointer: `left`/`top` are the box the press found,
            // so a board that scrolls under the tile does not drag it along.
            {
              position: "fixed",
              left: float.x,
              top: float.y,
              width: float.w,
              height: float.h,
              transform: `translate(${float.dx}px, ${float.dy}px)`,
            }
          : // A span is a measurement, not a look: the stored twelfths scaled onto this board.
            placement(widget, cols)
      }
    >
      <header
        className={`w__head${editing ? " w__head--grab" : ""}`}
        onPointerDown={(e) => {
          if (!editing) return;
          // A press on a tool is a click on that tool, not the start of a drag.
          if ((e.target as HTMLElement).closest(".wbtn")) return;
          startMove(e);
        }}
      >
        {editing && <DotsSixVerticalIcon className="w__grip" />}
        {!def.plain && <h2>{widgetTitle(i18n, widget, def)}</h2>}
        {meta && !def.plain && <span className="w__meta">{meta}</span>}
        {editing && (
          <div className="w__tools">
            <button type="button" className="wbtn" aria-label={t`Configure`} onClick={onConfig}>
              <GearIcon />
            </button>
            <button type="button" className="wbtn wbtn--danger" aria-label={t`Remove`} onClick={onRemove}>
              <TrashIcon />
            </button>
          </div>
        )}
      </header>
      <div className="w__body">
        <def.Render widget={widget} date={date} period={period} />
      </div>
      {editing &&
        HANDLES.map(({ key, dir }) => {
          const press = (e: React.PointerEvent) => {
            grab.current.dir = dir;
            startResize(e);
          };
          // The bottom-right corner is a button as well as a handle: a keyboard has no drag,
          // and one focus stop per tile is enough to resize it.
          return key === "se" ? (
            <button
              key={key}
              type="button"
              className="wgrab wgrab--se"
              aria-label={t`Resize`}
              onPointerDown={press}
              onKeyDown={(e) => {
                const step = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[
                  e.key
                ];
                if (!step) return;
                e.preventDefault();
                onResize(fitSize(def, widget.w + step[0], widget.h + step[1]), true);
              }}
            />
          ) : (
            <span key={key} className={`wgrab wgrab--${key}`} aria-hidden="true" onPointerDown={press} />
          );
        })}
    </section>
  );
}
