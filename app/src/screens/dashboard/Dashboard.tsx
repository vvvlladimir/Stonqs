import { Trans, useLingui } from "@lingui/react/macro";
import { Fragment, useRef, useState } from "react";
import { CheckIcon, PlusIcon, SlidersHorizontalIcon } from "@phosphor-icons/react";
import { Page } from "../../components/Page";
import { ErrorText, useToast } from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import {
  boardFromFile,
  boardToFile,
  newId,
  useUiState,
  type Dashboard as Board,
  type UiState,
  type Widget,
} from "../../lib/uiState";
import { downloadJson, fileNameOf, pickJsonFile } from "../../lib/configFile";
import { usePeriodRanges, type PeriodId } from "../../lib/periods";
import { BoardName, Config, DeleteBoard, Palette } from "./dialogs";
import { EditBar } from "./EditBar";
import {
  dropOrder,
  placement,
  sameOrder,
  useBoardColumns,
  useEdgeScroll,
  useReflow,
  type TileBox,
} from "./grid";
import { WidgetTile } from "./WidgetTile";
import { makeWidget } from "./widgets";
import { useAsOf } from "../../lib/asOf";

type Dialog =
  | { kind: "palette" }
  | { kind: "config"; widget: Widget }
  | { kind: "board"; action: "new" | "rename" }
  | { kind: "delete" };

export function Dashboard() {
  const { t } = useLingui();
  const date = useAsOf().date;
  const { ui, ready, loadError, save } = useUiState();
  const [editing, setEditing] = useState(false);
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");
  const ranges = usePeriodRanges(date);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  // The tile under the pointer: it leaves the flow and follows the cursor, and the slot it came
  // from stays behind as the hole that travels to where it would land.
  const [drag, setDrag] = useState<{ id: string; box: DOMRect; dx: number; dy: number } | null>(null);
  // The order under the pointer, kept out of the saved layout until the drag ends.
  const [order, setOrder] = useState<string[] | null>(null);
  // Where the pointer last was, so an edge scroll can re-ask the same question of a moved board.
  const point = useRef({ x: 0, y: 0 });
  // The size under the pointer, kept out of the saved layout until the drag ends: a resize
  // must not write settings on every pointer move.
  const [preview, setPreview] = useState<({ id: string } & TileBox) | null>(null);
  const gridRef = useRef<HTMLDivElement | null>(null);
  const cols = useBoardColumns();
  const toast = useToast();
  // A rejected file is a lasting message beside the board, not a toast that fades.
  const [importError, setImportError] = useState<string | null>(null);

  const board = ui.dashboards.find((d) => d.id === ui.active_dashboard) ?? ui.dashboards[0];

  const patch = (next: Partial<UiState>) => save({ ...ui, ...next });
  const setBoard = (widgets: Widget[]) =>
    patch({ dashboards: ui.dashboards.map((d) => (d.id === board.id ? { ...d, widgets } : d)) });

  const resize = (widget: Widget, box: TileBox, commit: boolean) => {
    if (!commit) {
      setPreview({ ...box, id: widget.id });
      return;
    }
    setPreview(null);
    const next = { ...widget, ...box };
    if (next.w === widget.w && next.h === widget.h && next.x === widget.x && next.y === widget.y) return;
    setBoard(board.widgets.map((w) => (w.id === widget.id ? next : w)));
  };

  /** What a tile is drawn at: the drag's preview while it lasts, the stored box otherwise. */
  const sized = (widget: Widget): Widget =>
    preview && preview.id === widget.id ? { ...widget, ...preview, id: widget.id } : widget;

  /** Widgets in the order they are drawn: the drag's own while it lasts, the stored one after. */
  const shown = order ? order.flatMap((id) => board.widgets.filter((w) => w.id === id)) : board.widgets;

  /** Re-ask where the dragged tile would land. Called on every move and on every scroll step. */
  const dragOver = (id: string) => {
    const grid = gridRef.current;
    if (!grid) return;
    const next = dropOrder(
      grid,
      order ?? board.widgets.map((w) => w.id),
      id,
      point.current.x,
      point.current.y,
    );
    setOrder((current) => (sameOrder(current, next) ? current : next));
  };

  // The board scrolls itself while a tile is held against the edge; after each step the drag
  // asks again, because the board moved under a pointer that did not.
  const edge = useEdgeScroll(gridRef, () => {
    if (drag) dragOver(drag.id);
  });

  const dropped = (commit: boolean) => {
    edge.stop();
    setDrag(null);
    // A dropped tile lands where the flow puts it: a pinned column or an empty strip above it
    // belonged to the place it left.
    if (commit && order)
      setBoard(shown.map((w) => (w.id === drag?.id ? { ...w, x: undefined, y: undefined } : w)));
    setOrder(null);
  };

  // Tiles that change place animate from where they were; only a move does, because a resize
  // already follows the pointer and a tile sliding under it would be a second opinion.
  useReflow(
    gridRef,
    shown
      .map((w) => sized(w))
      .map((w) => `${w.id}:${w.w}x${w.h}@${w.x ?? ""},${w.y ?? 0}`)
      .join("|"),
    drag !== null,
  );

  if (loadError) return <ErrorText error={loadError} />;
  if (!ready)
    return (
      <p className="muted">
        <Trans>Loading the layout…</Trans>
      </p>
    );

  return (
    <Page
      archetype="overview"
      controls={
        <>
          <div className="dtabs">
            {ui.dashboards.map((d) => (
              <button
                key={d.id}
                type="button"
                className={`dtab${d.id === board.id ? " dtab--active" : ""}`}
                onClick={() => patch({ active_dashboard: d.id })}
              >
                {d.name}
              </button>
            ))}
            <button
              type="button"
              className="dtab dtab--add"
              aria-label={t`New dashboard`}
              onClick={() => setDialog({ kind: "board", action: "new" })}
            >
              <PlusIcon />
            </button>
          </div>
          <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />
          <button
            type="button"
            className={`iconbtn${editing ? " iconbtn--on" : ""}`}
            onClick={() => setEditing(!editing)}
          >
            {editing ? <CheckIcon /> : <SlidersHorizontalIcon />}
            {editing ? t`Done` : t`Edit`}
          </button>
        </>
      }
      banner={
        editing ? (
          <EditBar
            board={board}
            onRename={() => setDialog({ kind: "board", action: "rename" })}
            onDuplicate={() => {
              const copy: Board = {
                id: newId("d"),
                name: t`${board.name} — copy`,
                widgets: board.widgets.map((w) => ({ ...w, id: newId("w"), cfg: { ...w.cfg } })),
              };
              patch({ dashboards: [...ui.dashboards, copy], active_dashboard: copy.id });
            }}
            onExport={() => downloadJson(fileNameOf(board.name, "dashboard"), boardToFile(board))}
            onImport={async () => {
              const text = await pickJsonFile();
              if (text === null) return;
              const imported = boardFromFile(text);
              if (!imported) {
                setImportError(t`That file is not a dashboard layout.`);
                return;
              }
              setImportError(null);
              patch({ dashboards: [...ui.dashboards, imported], active_dashboard: imported.id });
              toast(t`Layout imported.`);
            }}
            onDelete={() => setDialog({ kind: "delete" })}
          />
        ) : undefined
      }
    >
      <ErrorText>{importError}</ErrorText>

      <div
        className={`wgrid${editing ? " is-editing" : ""}${drag || preview ? " is-moving" : ""}`}
        ref={gridRef}
      >
        {shown.map((widget) => (
          <Fragment key={widget.id}>
            {drag?.id === widget.id && (
              // The hole the tile left: it is what reorders, so the landing place is visible
              // before the tile is dropped into it.
              <div
                className="wslot"
                data-id={widget.id}
                style={placement({ w: widget.w, h: widget.h }, cols)}
              />
            )}
            <WidgetTile
              widget={sized(widget)}
              cols={cols}
              gridRef={gridRef}
              date={date}
              period={period}
              editing={editing}
              float={
                drag?.id === widget.id
                  ? {
                      x: drag.box.left,
                      y: drag.box.top,
                      w: drag.box.width,
                      h: drag.box.height,
                      dx: drag.dx,
                      dy: drag.dy,
                    }
                  : null
              }
              onMoveStart={(box) => setDrag({ id: widget.id, box, dx: 0, dy: 0 })}
              onMove={(dx, dy, x, y) => {
                point.current = { x, y };
                setDrag((current) => (current ? { ...current, dx, dy } : current));
                edge.follow(y);
                dragOver(widget.id);
              }}
              onMoveEnd={dropped}
              onResize={(size, commit) => resize(widget, size, commit)}
              onConfig={() => setDialog({ kind: "config", widget })}
              onRemove={() => setBoard(board.widgets.filter((w) => w.id !== widget.id))}
            />
          </Fragment>
        ))}

        {editing && (
          <button type="button" className="wadd" onClick={() => setDialog({ kind: "palette" })}>
            <PlusIcon /> <Trans>Add widget</Trans>
          </button>
        )}
      </div>

      {board.widgets.length === 0 && !editing && (
        <div className="empty">
          <h3>
            <Trans>The dashboard is empty</Trans>
          </h3>
          <p>
            <Trans>
              Press "Edit" and add widgets — the overview is assembled around what you actually watch.
            </Trans>
          </p>
        </div>
      )}

      {dialog?.kind === "palette" && (
        <Palette
          onClose={() => setDialog(null)}
          onPick={(type) => {
            setBoard([...board.widgets, makeWidget(type, newId("w"))]);
            setDialog(null);
          }}
        />
      )}

      {dialog?.kind === "config" && (
        <Config
          widget={dialog.widget}
          onClose={() => setDialog(null)}
          onSave={(next) => {
            setBoard(board.widgets.map((w) => (w.id === next.id ? next : w)));
            setDialog(null);
          }}
        />
      )}

      {dialog?.kind === "board" && (
        <BoardName
          action={dialog.action}
          current={dialog.action === "rename" ? board.name : ""}
          onClose={() => setDialog(null)}
          onSave={(name) => {
            if (dialog.action === "rename") {
              patch({ dashboards: ui.dashboards.map((d) => (d.id === board.id ? { ...d, name } : d)) });
            } else {
              const created: Board = { id: newId("d"), name, widgets: [] };
              patch({ dashboards: [...ui.dashboards, created], active_dashboard: created.id });
              setEditing(true);
            }
            setDialog(null);
          }}
        />
      )}

      {dialog?.kind === "delete" && (
        <DeleteBoard
          board={board}
          boards={ui.dashboards}
          onClose={() => setDialog(null)}
          onDelete={() => {
            const rest = ui.dashboards.filter((d) => d.id !== board.id);
            patch({ dashboards: rest, active_dashboard: rest[0].id });
            setDialog(null);
          }}
        />
      )}
    </Page>
  );
}
