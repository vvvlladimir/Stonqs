import { useLingui } from "@lingui/react/macro";
import { GROUPS } from "../../components/domain/positionColumns";
import { ColumnPicker } from "../../components/ui";
import { DEFAULT_UI, useUiState } from "../../lib/uiState";
import type { useWatchColumns } from "./columns";

/** Which columns the watch table shows; the stored array is the display order. */
export function WatchColumnPicker({
  columns,
  selected,
  currency,
  onClose,
}: {
  columns: ReturnType<typeof useWatchColumns>;
  selected: string[];
  currency: string;
  onClose: () => void;
}) {
  const { i18n } = useLingui();
  const { save } = useUiState();
  return (
    <ColumnPicker
      columns={columns.map((c) => ({
        id: c.id,
        label: c.label(i18n, currency),
        group: c.group,
        tip: c.tip?.(i18n),
      }))}
      groups={GROUPS.map((g) => ({ id: g.id, label: i18n._(g.label) }))}
      selected={selected}
      onChange={(watch_columns) => save((ui) => ({ ...ui, watch_columns }))}
      onReset={() => save((ui) => ({ ...ui, watch_columns: DEFAULT_UI.watch_columns }))}
      onClose={onClose}
    />
  );
}
