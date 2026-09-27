import { useLingui } from "@lingui/react/macro";
import { PlusIcon } from "@phosphor-icons/react";
import type { Dashboard as Board } from "../../lib/uiState";

export function BoardTabs({
  boards,
  active,
  onPick,
  onNew,
}: {
  boards: Board[];
  active: string;
  onPick: (id: string) => void;
  onNew: () => void;
}) {
  const { t } = useLingui();
  return (
    <div className="dtabs">
      {boards.map((d) => (
        <button
          key={d.id}
          type="button"
          className={`dtab${d.id === active ? " dtab--active" : ""}`}
          onClick={() => onPick(d.id)}
        >
          {d.name}
        </button>
      ))}
      <button type="button" className="dtab dtab--add" aria-label={t`New dashboard`} onClick={onNew}>
        <PlusIcon />
      </button>
    </div>
  );
}
