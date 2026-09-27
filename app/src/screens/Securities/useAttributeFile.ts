import { useState } from "react";
import { open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { affects, useInvalidate } from "../../lib/queries";
import type { AttributePreview } from "../../lib/types";

const CSV = [{ name: "CSV", extensions: ["csv"] }];

/** The attribute CSV round trip: pick a file, preview it, commit it, or export the current set. */
export function useAttributeFile() {
  const invalidate = useInvalidate();
  // Kept beside its preview: the commit reads the file again, so the plan shown and the plan
  // written come from the same bytes.
  const [file, setFile] = useState<{ path: string; preview: AttributePreview } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const commit = useMutation({
    mutationFn: (path: string) => api.attributesImportCommitPath(path),
    onSuccess: () => {
      setFile(null);
      invalidate(...affects.securities);
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
      setFile({ path, preview: await api.attributesImportPreviewPath(path) });
    });

  const exportAll = () =>
    attempt(async () => {
      const path = await saveFile({ defaultPath: "instrument-attributes.csv", filters: CSV });
      if (path) await api.attributesExportSave(path);
    });

  const importFile = () => {
    if (file) commit.mutate(file.path);
  };

  return {
    file,
    error,
    pick,
    exportAll,
    importFile,
    importing: commit.isPending,
    close: () => setFile(null),
  };
}
