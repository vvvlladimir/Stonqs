import { useState } from "react";
import { PlusIcon } from "@phosphor-icons/react";
import { useLingui } from "@lingui/react/macro";
import { Seg } from "../ui";
import { BUILTIN_PRESETS, periodLabel, type PeriodId } from "../../lib/periods";
import { PeriodEditor } from "./PeriodEditor";
import type { PeriodRange } from "../../lib/types";

/**
 * The one period axis of the app. Every period is resolved to dates by the core, so a screen
 * never computes a window itself; `ranges` is what this portfolio can actually show — the
 * shipped presets the user kept, plus the periods they added.
 */
export function PeriodControl({
  value,
  onChange,
  ranges,
}: {
  value: PeriodId;
  onChange: (id: PeriodId) => void;
  ranges?: PeriodRange[];
}) {
  const { t, i18n } = useLingui();
  const [editing, setEditing] = useState(false);
  // Before the ranges arrive the shipped seven stand in, so the strip does not jump in width.
  const options = ranges ?? BUILTIN_PRESETS.map((id) => ({ id, name: null, from: "", to: "" }));

  return (
    // A box of its own: the strip sits at the end of the controls row on every screen, and it
    // is what scrolls when the row runs out of width, so whatever stands beside it stays put.
    <div className="period">
      <Seg
        label={t`Period`}
        value={value}
        onChange={onChange}
        options={options.map((range) => ({ value: range.id, label: periodLabel(i18n, range) }))}
        end={
          <button
            type="button"
            aria-label={t`Add a period`}
            title={t`Add or remove periods`}
            onClick={() => setEditing(true)}
          >
            <PlusIcon />
          </button>
        }
      />
      {editing && <PeriodEditor onClose={() => setEditing(false)} />}
    </div>
  );
}
