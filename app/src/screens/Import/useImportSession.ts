import { useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { api, ApiError } from "../../lib/api";
import { affects, useAccounts, useImportTemplates, useInvalidate, usePlugins } from "../../lib/queries";
import type {
  ImportMapping,
  ImportOptions,
  ImportPreviewData,
  ImportResult,
  ParseConfig,
  Plugin,
  RowOverride,
  Unlock,
} from "../../lib/types";

/** No settings at all: the core detects every one of them from the file itself. */
const BLANK_CONFIG: ParseConfig = {
  delimiter: null,
  skip_top_rows: 0,
  skip_bottom_rows: 0,
  has_header: null,
  date_format: null,
  decimal_separator: null,
};

type Detected = { config: ParseConfig; mapping: ImportMapping };

/** One import from file to commit; every layout change re-asks the core for the preview. */
export function useImportSession() {
  const invalidate = useInvalidate();
  const layout = useLayout();
  const [fileName, setFileName] = useState<string | null>(null);
  const [options, setOptions] = useState<ImportOptions>({
    create_missing_securities: true,
    new_security_kind: "OTHER",
    import_duplicates: false,
    import_similar: false,
  });
  const [result, setResult] = useState<ImportResult | null>(null);
  const plugins = usePlugins();

  // A plugin's reader recognised a sealed file: which file, which reader asked, and whether a
  // password was already refused. The password itself lives only in the dialog.
  const [sealed, setSealed] = useState<{ path: string; reader: string; tried: boolean } | null>(null);

  const load = useMutation({
    mutationFn: ({ path, unlock }: { path: string; unlock?: Unlock }) => api.importLoadPath(path, unlock),
    onError: (error, { path }) => {
      if (error instanceof ApiError && error.detail.code === "file_protected") {
        setSealed({ path, reader: error.detail.reader, tried: error.detail.tried });
      }
    },
    onSuccess: (data) => {
      setSealed(null);
      setResult(null);
      layout.adopt(data);
    },
  });

  const commit = useMutation({
    mutationFn: () => api.importCommit(layout.config!, layout.mapping, layout.overrides, options),
    onSuccess: (data) => {
      setResult(data);
      invalidate(...affects.transactions);
    },
  });

  const pickFile = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Broker export", extensions: readableExtensions(plugins.data?.plugins ?? []) }],
    });
    if (typeof path !== "string") return;
    setFileName(path.split("/").pop() ?? path);
    load.mutate({ path });
  };

  const reset = () => {
    api.importClear();
    layout.adopt(null);
    setResult(null);
    setFileName(null);
  };

  const { adopt: _, ...shown } = layout;
  return {
    ...shown,
    fileName,
    options,
    setOptions,
    result,
    sealed,
    setSealed,
    plugins,
    load,
    commit,
    pickFile,
    reset,
  };
}

/** How the file is read and laid out, and the preview that reading gives. */
function useLayout() {
  const [step, setStep] = useState(0);
  const [preview, setPreview] = useState<ImportPreviewData | null>(null);
  // What the core made of the file on its own. Kept so that picking a template — which
  // answers every question at once, and may answer them for the wrong broker — stays undoable.
  const [detected, setDetected] = useState<Detected | null>(null);
  const [config, setConfig] = useState<ParseConfig | null>(null);
  const [mapping, setMapping] = useState<ImportMapping | null>(null);
  const [overrides, setOverrides] = useState<RowOverride[]>([]);
  const [template, setTemplate] = useState("");
  const accounts = useAccounts();
  const templates = useImportTemplates();

  // Each change of the layout asks for a preview of its own, and two answers may come back in
  // the order the host finished them rather than the order they were asked in. The table would
  // then show a reading of a layout the user has already moved on from, so an answer older than
  // the question in hand is dropped.
  const asked = useRef(0);
  const refresh = useMutation({
    mutationFn: async (next: {
      config: ParseConfig;
      mapping: ImportMapping | null;
      overrides: RowOverride[];
    }) => {
      const ticket = ++asked.current;
      const data = await api.importPreview(next.config, next.mapping, next.overrides);
      return { ticket, data };
    },
    onSuccess: ({ ticket, data }) => {
      if (ticket === asked.current) setPreview(data);
    },
  });

  /** A freshly loaded file, or none. */
  const adopt = (data: ImportPreviewData | null) => {
    setPreview(data);
    // A recognised file arrives already laid out, so what the core would have detected on
    // its own is not in hand — "— detect —" asks for it again rather than replaying it.
    setDetected(data && !data.applied_template ? { config: data.config, mapping: data.mapping } : null);
    setConfig(data?.config ?? null);
    setMapping(data?.mapping ?? null);
    setOverrides([]);
    setTemplate(data?.applied_template ?? "");
    setStep(0);
  };

  // Recompute the preview after each mapping change.
  const apply = (nextConfig: ParseConfig, nextMapping: ImportMapping | null, next: RowOverride[]) => {
    setConfig(nextConfig);
    setMapping(nextMapping);
    setOverrides(next);
    refresh.mutate({ config: nextConfig, mapping: nextMapping, overrides: next });
  };

  const current = mapping ?? preview?.mapping ?? null;

  /** Lay the file out by a saved template, or — with no id — by what the core detected. */
  const applyTemplate = (id: string) => {
    setTemplate(id);
    const account = current?.account_id ?? null;
    const found = templates.data?.find((t) => t.id === id);
    if (!found) {
      if (detected) apply(detected.config, { ...detected.mapping, account_id: account }, overrides);
      else apply(BLANK_CONFIG, null, overrides);
      return;
    }
    // The saved account comes back with the layout, unless it has since been deleted —
    // a dangling id would look chosen while nothing lands on it.
    const saved = found.mapping.account_id;
    const kept = accounts.data?.some((a) => a.id === saved) ? saved : account;
    apply(found.config, { ...found.mapping, account_id: kept }, overrides);
  };

  return {
    step,
    setStep,
    preview,
    config,
    mapping,
    current,
    overrides,
    template,
    setTemplate,
    accounts,
    templates,
    refresh,
    apply,
    applyTemplate,
    adopt,
  };
}

/** A plugin's reader is only reachable if its files can be picked, so the filter is the app's own
 *  endings plus whatever the installed readers say they read. */
function readableExtensions(plugins: Plugin[]): string[] {
  const fromPlugins = plugins
    .filter((plugin) => plugin.status === "ok")
    .flatMap((plugin) => plugin.readers)
    .flatMap((reader) => reader.extensions)
    .map((extension) => extension.replace(/^\./, "").toLowerCase());
  return [...new Set(["csv", "txt", "xml", "json", ...fromPlugins])];
}
