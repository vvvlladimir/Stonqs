import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { save as saveFile } from "@tauri-apps/plugin-dialog";
import { api, today } from "../../lib/api";
import { usePlugins } from "../../lib/queries";
import type { useMenu } from "../../components/ui";
import type { InstalledWriter, TransactionFilter } from "../../lib/types";

/** The app's own file, or a plugin's format written from it; the screen's filter, not the whole journal. */
export function useExport(filter: TransactionFilter, menu: ReturnType<typeof useMenu>) {
  const { t } = useLingui();
  const plugins = usePlugins();
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const writers = plugins.data?.writers ?? [];

  const save = async (writer: InstalledWriter | null) => {
    const extension = writer?.extension ?? "json";
    const path = await saveFile({
      defaultPath: `transactions-${today()}.${extension}`,
      filters: [{ name: writer?.name ?? "Stonqs transactions", extensions: [extension] }],
    });
    if (!path) return;
    setExporting(true);
    setError(null);
    try {
      await api.transactionsExportSave(filter, path, writer?.key ?? null);
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setExporting(false);
    }
  };

  /** With no plugin format installed there is nothing to choose, so the button saves at once. */
  const choose = (el: HTMLElement) => {
    if (writers.length === 0) return void save(null);
    menu.openFrom(
      "export",
      [
        { label: t`Stonqs file`, onSelect: () => void save(null) },
        ...writers.map((writer) => ({ label: writer.name, onSelect: () => void save(writer) })),
      ],
      el,
    );
  };

  return { exporting, error, choose, saveOwn: () => save(null) };
}
