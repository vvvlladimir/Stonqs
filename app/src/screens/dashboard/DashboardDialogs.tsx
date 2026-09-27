import { newId, type Dashboard as Board, type UiState, type Widget } from "../../lib/uiState";
import { BoardName, Config, DeleteBoard, Palette } from "./dialogs";
import { makeWidget, useWidgetCatalog } from "./widgets";

export type Dialog =
  | { kind: "palette" }
  | { kind: "config"; widget: Widget }
  | { kind: "board"; action: "new" | "rename" }
  | { kind: "delete" };

/** Whichever dialog the board has open, each closing itself once it has written its change. */
export function DashboardDialogs({
  dialog,
  board,
  boards,
  patch,
  setBoard,
  onCreated,
  onClose,
}: {
  dialog: Dialog | null;
  board: Board;
  boards: Board[];
  patch: (next: Partial<UiState>) => void;
  setBoard: (widgets: Widget[]) => void;
  /** A new board opens in edit mode: it has nothing to show yet. */
  onCreated: () => void;
  onClose: () => void;
}) {
  const catalog = useWidgetCatalog();
  const done = (write: () => void) => {
    write();
    onClose();
  };

  switch (dialog?.kind) {
    case "palette":
      return (
        <Palette
          onClose={onClose}
          onPick={(type) =>
            done(() => {
              const def = catalog.of(type);
              if (def) setBoard([...board.widgets, makeWidget(type, newId("w"), def)]);
            })
          }
        />
      );
    case "config":
      return (
        <Config
          widget={dialog.widget}
          onClose={onClose}
          onSave={(next) => done(() => setBoard(board.widgets.map((w) => (w.id === next.id ? next : w))))}
        />
      );
    case "board":
      return (
        <BoardName
          action={dialog.action}
          current={dialog.action === "rename" ? board.name : ""}
          onClose={onClose}
          onSave={(name) =>
            done(() => {
              if (dialog.action === "rename") {
                patch({ dashboards: boards.map((d) => (d.id === board.id ? { ...d, name } : d)) });
              } else {
                const created: Board = { id: newId("d"), name, widgets: [] };
                patch({ dashboards: [...boards, created], active_dashboard: created.id });
                onCreated();
              }
            })
          }
        />
      );
    case "delete":
      return (
        <DeleteBoard
          board={board}
          boards={boards}
          onClose={onClose}
          onDelete={() =>
            done(() => {
              const rest = boards.filter((d) => d.id !== board.id);
              patch({ dashboards: rest, active_dashboard: rest[0].id });
            })
          }
        />
      );
    default:
      return null;
  }
}
