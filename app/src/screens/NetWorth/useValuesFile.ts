import { useState } from "react";
import { open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { affects, useInvalidate } from "../../lib/queries";
import type { ValuesPreview } from "../../lib/types";

const CSV = [{ name: "CSV", extensions: ["csv"] }];

/** The valuations CSV round trip: pick a file, see what it would write, write it, or export. */
export function useValuesFile() {
  const invalidate = useInvalidate();
  // The path is kept beside its preview because the commit reads the file again: what was shown
  // and what is written come from the same bytes.
  const [file, setFile] = useState<{ path: string; preview: ValuesPreview } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const commit = useMutation({
    mutationFn: (path: string) => api.assetValuesImportCommitPath(path),
    onSuccess: () => {
      setFile(null);
      invalidate(...affects.assets);
    },
  });

  const attempt = async (run: () => Promise<void>) => {
    setError(null);
    try {
      await run();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const pick = () =>
    attempt(async () => {
      const path = await openFile({ multiple: false, filters: CSV });
      if (typeof path !== "string") return;
      setFile({ path, preview: await api.assetValuesImportPreviewPath(path) });
    });

  const exportAll = () =>
    attempt(async () => {
      const path = await saveFile({ defaultPath: "valuations.csv", filters: CSV });
      if (path) await api.assetValuesExportSave(path);
    });

  return {
    file,
    error,
    pick,
    exportAll,
    write: () => file && commit.mutate(file.path),
    writing: commit.isPending,
    close: () => setFile(null),
  };
}
