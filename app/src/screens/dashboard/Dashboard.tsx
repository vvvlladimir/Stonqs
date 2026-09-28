import { Trans, useLingui } from "@lingui/react/macro";
import { Fragment, useRef, useState } from "react";
import { CheckIcon, PlusIcon, SlidersHorizontalIcon } from "@phosphor-icons/react";
import { Page } from "../../components/Page";
import { ErrorText, useToast } from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { useAsOf } from "../../lib/asOf";
import { downloadJson, fileNameOf, pickJsonFile } from "../../lib/configFile";
import { usePeriodRanges, type PeriodId } from "../../lib/periods";
import {
  boardFromFile,
  boardToFile,
  newId,
  useUiState,
  type Dashboard as Board,
  type UiState,
  type Widget,
} from "../../lib/uiState";
import { BoardTabs } from "./BoardTabs";
import { DashboardDialogs, type Dialog } from "./DashboardDialogs";
import { EditBar } from "./EditBar";
import { limitsOf, placement, useBoardColumns } from "./grid";
import { useBoardDrag } from "./useBoardDrag";
import { WidgetTile } from "./WidgetTile";
import { useWidgetCatalog } from "./widgets";

export function Dashboard() {
  const { t } = useLingui();
  const date = useAsOf().date;
  const { ui, ready, loadError, save } = useUiState();
  const [editing, setEditing] = useState(false);
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");
  const ranges = usePeriodRanges(date);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const catalog = useWidgetCatalog();
  const gridRef = useRef<HTMLDivElement | null>(null);
  // The board, not the window: the grid element is what has the columns.
  const cols = useBoardColumns(gridRef);

  const board = ui.dashboards.find((d) => d.id === ui.active_dashboard) ?? ui.dashboards[0];

  const patch = (next: Partial<UiState>) => save((ui) => ({ ...ui, ...next }));
  const setBoard = (widgets: Widget[]) =>
    patch({ dashboards: ui.dashboards.map((d) => (d.id === board.id ? { ...d, widgets } : d)) });

  const tiles = useBoardDrag(board.widgets, setBoard, gridRef);
  const files = useBoardFiles(board, ui.dashboards, patch);

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
          <BoardTabs
            boards={ui.dashboards}
            active={board.id}
            onPick={(id) => patch({ active_dashboard: id })}
            onNew={() => setDialog({ kind: "board", action: "new" })}
          />
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
            onDuplicate={files.duplicate}
            onExport={files.exportBoard}
            onImport={files.importBoard}
            onDelete={() => setDialog({ kind: "delete" })}
          />
        ) : undefined
      }
    >
      <ErrorText>{files.importError}</ErrorText>

      {/* `wboard` is the container the tiles' own stylesheet measures, so the columns the
          layout draws and the columns this file counts are one width. */}
      <div className="wboard">
        <div
          className={`wgrid${editing ? " is-editing" : ""}${tiles.moving ? " is-moving" : ""}`}
          ref={gridRef}
        >
          {tiles.shown.map((widget) => (
            <Fragment key={widget.id}>
              {tiles.drag?.id === widget.id && catalog.of(widget.type) && (
                // The hole the tile left: it is what reorders, so the landing place is visible
                // before the tile is dropped into it.
                <div
                  className="wslot"
                  data-id={widget.id}
                  style={placement(
                    { w: widget.w, h: widget.h },
                    cols,
                    limitsOf(widget, catalog.of(widget.type)!).w,
                  )}
                />
              )}
              <WidgetTile
                widget={tiles.sized(widget)}
                cols={cols}
                gridRef={gridRef}
                date={date}
                period={period}
                editing={editing}
                float={
                  tiles.drag?.id === widget.id
                    ? {
                        x: tiles.drag.box.left,
                        y: tiles.drag.box.top,
                        w: tiles.drag.box.width,
                        h: tiles.drag.box.height,
                        dx: tiles.drag.dx,
                        dy: tiles.drag.dy,
                      }
                    : null
                }
                onMoveStart={(box) => tiles.start(widget.id, box)}
                onMove={(dx, dy, x, y) => tiles.move(widget.id, dx, dy, x, y)}
                onMoveEnd={tiles.drop}
                onResize={(size, commit) => tiles.resize(widget, size, commit)}
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

      <DashboardDialogs
        dialog={dialog}
        board={board}
        boards={ui.dashboards}
        patch={patch}
        setBoard={setBoard}
        onCreated={() => setEditing(true)}
        onClose={() => setDialog(null)}
      />
    </Page>
  );
}

/** A board copied, written to a file, or read from one. A rejected file is a lasting message
 *  beside the board, not a toast that fades. */
function useBoardFiles(board: Board, boards: Board[], patch: (next: Partial<UiState>) => void) {
  const { t } = useLingui();
  const toast = useToast();
  const [importError, setImportError] = useState<string | null>(null);

  const duplicate = () => {
    const copy: Board = {
      id: newId("d"),
      name: t`${board.name} — copy`,
      widgets: board.widgets.map((w) => ({ ...w, id: newId("w"), cfg: { ...w.cfg } })),
    };
    patch({ dashboards: [...boards, copy], active_dashboard: copy.id });
  };

  const exportBoard = () => downloadJson(fileNameOf(board.name, "dashboard"), boardToFile(board));

  const importBoard = async () => {
    const text = await pickJsonFile();
    if (text === null) return;
    const imported = boardFromFile(text);
    if (!imported) {
      setImportError(t`That file is not a dashboard layout.`);
      return;
    }
    setImportError(null);
    patch({ dashboards: [...boards, imported], active_dashboard: imported.id });
    toast(t`Layout imported.`);
  };

  return { duplicate, exportBoard, importBoard, importError };
}
