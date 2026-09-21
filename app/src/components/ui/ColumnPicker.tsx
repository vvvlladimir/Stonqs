import { useRef, useState } from "react";
import { ArrowCounterClockwiseIcon, DotsSixVerticalIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";
import { usePointerDrag } from "../../lib/pointerDrag";
import { Modal } from "./Modal";
import { SearchBox } from "./SearchBox";

export interface ColumnChoice {
  id: string;
  label: string;
  /** Group heading the choice is filed under; ungrouped choices are listed first. */
  group?: string;
  /** One short sentence saying what the figure is. */
  tip?: string;
}

export interface ColumnGroup {
  id: string;
  label: string;
}

/**
 * Which columns a table shows **and in which order**. The shown ones are the list on top, in
 * the order they are shown, dragged by the handle; the rest sit under their group heading.
 * The caller stores the array as given — it is the display order, not a filter of a catalogue.
 */
export function ColumnPicker({
  columns,
  groups,
  selected,
  onChange,
  onReset,
  onClose,
}: {
  columns: ColumnChoice[];
  groups?: ColumnGroup[];
  selected: string[];
  onChange: (selected: string[]) => void;
  /** Offers "Reset to default" when the caller knows what its default is. */
  onReset?: () => void;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const [query, setQuery] = useState("");
  const byId = new Map(columns.map((column) => [column.id, column]));
  const shown = selected.map((id) => byId.get(id)).filter((c): c is ColumnChoice => c !== undefined);

  const needle = query.trim().toLowerCase();
  const matches = (column: ColumnChoice) => !needle || column.label.toLowerCase().includes(needle);
  const rest = columns.filter((column) => !selected.includes(column.id) && matches(column));

  const toggle = (id: string, on: boolean) =>
    onChange(on ? [...selected, id] : selected.filter((other) => other !== id));

  return (
    <Modal title={t`Columns`} onClose={onClose}>
      <SearchBox value={query} onChange={setQuery} placeholder={t`Find a column`} />

      {shown.length > 0 && (
        <>
          <p className="group-label">
            <Trans>Shown, in order</Trans>
          </p>
          <Shown columns={columns} order={selected} onReorder={onChange} onToggle={toggle} />
        </>
      )}

      {groups
        ? groups.map((group) => {
            const members = rest.filter((column) => column.group === group.id);
            if (members.length === 0) return null;
            return (
              <div key={group.id}>
                <p className="group-label">{group.label}</p>
                <div className="colpick">
                  {members.map((column) => (
                    <Choice key={column.id} column={column} on={false} onToggle={toggle} />
                  ))}
                </div>
              </div>
            );
          })
        : rest.length > 0 && (
            <div className="colpick">
              {rest.map((column) => (
                <Choice key={column.id} column={column} on={false} onToggle={toggle} />
              ))}
            </div>
          )}

      {onReset && (
        <button type="button" className="btn btn--ghost colpick__reset" onClick={onReset}>
          <ArrowCounterClockwiseIcon /> <Trans>Reset to default</Trans>
        </button>
      )}
    </Modal>
  );
}

/** A checkbox row; the label carries the column's own one-sentence explanation. */
function Choice({
  column,
  on,
  onToggle,
}: {
  column: ColumnChoice;
  on: boolean;
  onToggle: (id: string, on: boolean) => void;
}) {
  return (
    <label data-tip={column.tip}>
      <span>{column.label}</span>
      <input type="checkbox" checked={on} onChange={(e) => onToggle(column.id, e.target.checked)} />
    </label>
  );
}

/**
 * The shown columns, reordered by dragging a handle. The row under the pointer is found by
 * hit-testing rather than by arithmetic on heights, so a wrapped label does not throw the
 * order off; the order is followed in local state and written once the drag ends, the way
 * the dashboard moves a tile. The handle is also a button, so the same move is one arrow key
 * away.
 */
function Shown({
  columns,
  order,
  onReorder,
  onToggle,
}: {
  columns: ColumnChoice[];
  order: string[];
  onReorder: (order: string[]) => void;
  onToggle: (id: string, on: boolean) => void;
}) {
  const { t } = useLingui();
  const list = useRef<HTMLDivElement>(null);
  // Which handle was pressed, and the order the drag has reached: one gesture serves the whole
  // list, because a hook per row would be a hook count that changes with the choice.
  const held = useRef<string | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const [draft, setDraft] = useState<string[] | null>(null);
  const live = draft ?? order;
  const byId = new Map(columns.map((column) => [column.id, column]));
  const shown = live.map((id) => byId.get(id)).filter((c): c is ColumnChoice => c !== undefined);

  const moved = (from: string[], id: string, to: number): string[] => {
    const at = from.indexOf(id);
    if (at < 0 || to < 0 || to >= from.length || to === at) return from;
    const next = [...from];
    next.splice(to, 0, next.splice(at, 1)[0]);
    return next;
  };

  const overAt = (x: number, y: number): number => {
    const box = list.current;
    const under = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-col]");
    if (!box || !under || !box.contains(under)) return -1;
    return live.indexOf(under.dataset.col ?? "");
  };

  const drag = usePointerDrag({
    slop: 4,
    onStart: () => setDragging(held.current),
    onMove: (_dx, _dy, x, y) => {
      const id = held.current;
      if (id === null) return;
      const to = overAt(x, y);
      if (to >= 0) setDraft((from) => moved(from ?? order, id, to));
    },
    onEnd: (commit) => {
      if (commit && draft) onReorder(draft);
      held.current = null;
      setDragging(null);
      setDraft(null);
    },
  });

  return (
    <div className="colpick colpick--order" ref={list}>
      {shown.map((column, index) => (
        <label
          key={column.id}
          data-col={column.id}
          data-tip={column.tip}
          className={dragging === column.id ? "is-dragging" : undefined}
        >
          <span>
            <button
              type="button"
              className="colpick__grab"
              aria-label={t`Move "${column.label}"`}
              onPointerDown={(e) => {
                held.current = column.id;
                drag(e);
              }}
              onKeyDown={(e) => {
                if (e.key === "ArrowUp") onReorder(moved(order, column.id, index - 1));
                else if (e.key === "ArrowDown") onReorder(moved(order, column.id, index + 1));
                else return;
                e.preventDefault();
              }}
            >
              <DotsSixVerticalIcon aria-hidden />
            </button>
            {column.label}
          </span>
          <input type="checkbox" checked onChange={(e) => onToggle(column.id, e.target.checked)} />
        </label>
      ))}
    </div>
  );
}
