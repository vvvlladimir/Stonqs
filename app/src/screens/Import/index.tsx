import { Trans, useLingui } from "@lingui/react/macro";
import { Command } from "../../lib/commands";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeftIcon, ArrowRightIcon, CheckIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { affects, useAccounts, useImportTemplates, useInvalidate, usePlugins } from "../../lib/queries";
import { Page } from "../../components/Page";
import { Banner, Buttons, Chip, QueryError } from "../../components/ui";
import type {
  ImportMapping,
  ImportOptions,
  ImportPreviewData,
  ImportResult,
  ParseConfig,
  RowOverride,
} from "../../lib/types";
import { AssetsStep } from "./AssetsStep";
import { CommitStep } from "./CommitStep";
import { FileStep } from "./FileStep";
import { ParseStep } from "./ParseStep";

import { STEPS, fieldLabel, missingFields } from "./labels";

const LAST = STEPS.length - 1;

/** No settings at all: the core detects every one of them from the file itself. */
const BLANK_CONFIG: ParseConfig = {
  delimiter: null,
  skip_top_rows: 0,
  skip_bottom_rows: 0,
  has_header: null,
  date_format: null,
  decimal_separator: null,
};

export function Import() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const [step, setStep] = useState(0);
  const [fileName, setFileName] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportPreviewData | null>(null);
  // What the core made of the file on its own. Kept so that picking a template — which
  // answers every question at once, and may answer them for the wrong broker — stays undoable.
  const [detected, setDetected] = useState<{ config: ParseConfig; mapping: ImportMapping } | null>(null);
  const [config, setConfig] = useState<ParseConfig | null>(null);
  const [mapping, setMapping] = useState<ImportMapping | null>(null);
  const [overrides, setOverrides] = useState<RowOverride[]>([]);
  const [template, setTemplate] = useState("");
  const [options, setOptions] = useState<ImportOptions>({
    create_missing_securities: true,
    new_security_kind: "OTHER",
    import_duplicates: false,
    import_similar: false,
  });
  const [result, setResult] = useState<ImportResult | null>(null);

  const accounts = useAccounts();
  const templates = useImportTemplates();
  const plugins = usePlugins();

  const load = useMutation({
    mutationFn: api.importLoadPath,
    onSuccess: (data) => {
      setPreview(data);
      // A recognised file arrives already laid out, so what the core would have detected on
      // its own is not in hand — "— detect —" asks for it again rather than replaying it.
      setDetected(data.applied_template ? null : { config: data.config, mapping: data.mapping });
      setConfig(data.config);
      setMapping(data.mapping);
      setOverrides([]);
      setResult(null);
      setTemplate(data.applied_template ?? "");
      setStep(0);
    },
  });

  const refresh = useMutation({
    mutationFn: (next: { config: ParseConfig; mapping: ImportMapping | null; overrides: RowOverride[] }) =>
      api.importPreview(next.config, next.mapping, next.overrides),
    onSuccess: (data) => setPreview(data),
  });

  const commit = useMutation({
    mutationFn: () => api.importCommit(config!, mapping, overrides, options),
    onSuccess: (data) => {
      setResult(data);
      invalidate(...affects.transactions);
    },
  });

  // Recompute the preview after each mapping change.
  const apply = (nextConfig: ParseConfig, nextMapping: ImportMapping | null, next: RowOverride[]) => {
    setConfig(nextConfig);
    setMapping(nextMapping);
    setOverrides(next);
    refresh.mutate({ config: nextConfig, mapping: nextMapping, overrides: next });
  };

  const pickFile = async () => {
    // A plugin's reader is only reachable if its files can be picked, so the filter is the app's
    // own endings plus whatever the installed readers say they read.
    const fromPlugins = (plugins.data?.plugins ?? [])
      .filter((plugin) => plugin.status === "ok")
      .flatMap((plugin) => plugin.readers)
      .flatMap((reader) => reader.extensions)
      .map((extension) => extension.replace(/^\./, "").toLowerCase());
    const extensions = [...new Set(["csv", "txt", "xml", "json", ...fromPlugins])];
    const path = await open({
      multiple: false,
      filters: [{ name: "Broker export", extensions }],
    });
    if (typeof path !== "string") return;
    setFileName(path.split("/").pop() ?? path);
    load.mutate(path);
  };

  const current = mapping ?? preview?.mapping ?? null;
  const account = current?.account_id ?? null;

  /** Lay the file out by a saved template, or — with no id — by what the core detected. */
  const applyTemplate = (id: string) => {
    setTemplate(id);
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

  const reset = () => {
    api.importClear();
    setPreview(null);
    setDetected(null);
    setConfig(null);
    setMapping(null);
    setOverrides([]);
    setResult(null);
    setFileName(null);
    setTemplate("");
    setStep(0);
  };

  const ready = Boolean(config && preview && current);
  const missing = ready ? missingFields(current) : [];

  // The two hard stops of the wizard: rows with no account to land on, and required
  // columns nobody has pointed at — past either of them nothing parses at all.
  const blockedWhy =
    !ready || step !== 1
      ? undefined
      : missing.length > 0
        ? t`Point at these columns first: ${missing.map((field) => fieldLabel(i18n, field)).join(", ")}`
        : !current!.account_id
          ? t`Choose a default account first: otherwise the rows have nowhere to go.`
          : undefined;
  const blocked = blockedWhy !== undefined;
  const reachable = (i: number) => i === 0 || (ready && (i <= step || !blocked));

  return (
    <Page
      archetype="wizard"
      title={t`CSV import`}
      note={fileName ?? undefined}
      lead={i18n._(STEPS[step].hint)}
      steps={STEPS.map(({ title }, i) => (
        <Chip
          key={i}
          active={i === step}
          disabled={!reachable(i)}
          title={i > step && blocked ? blockedWhy : undefined}
          onClick={() => setStep(i)}
        >
          {i < step ? <CheckIcon weight="bold" /> : `${i + 1}.`} {i18n._(title)}
        </Chip>
      ))}
      banner={refresh.isError ? <QueryError error={refresh.error} /> : undefined}
      foot={
        <>
          <Buttons>
            {step > 0 && (
              <button className="btn btn--ghost" onClick={() => setStep(step - 1)}>
                <ArrowLeftIcon /> <Trans>Back</Trans>
              </button>
            )}
            {/* One wording for the whole wizard. It appears only once the file is read:
                until then there is nothing to move on to. */}
            {ready && step < LAST && (
              <button className="btn" disabled={blocked} title={blockedWhy} onClick={() => setStep(step + 1)}>
                <Trans>Next</Trans> <ArrowRightIcon />
              </button>
            )}
            {ready && (
              <button className="btn btn--ghost" onClick={reset}>
                <Trans>Start over</Trans>
              </button>
            )}
          </Buttons>
        </>
      }
    >
      {blocked && <Banner>{blockedWhy}</Banner>}

      <Command id="importFile" run={pickFile} disabled={step !== 0 || load.isPending} />
      {step === 0 && (
        <FileStep
          loading={load.isPending}
          onPick={pickFile}
          error={load.error}
          preview={preview}
          config={config}
          mapping={current}
        />
      )}

      {ready && step === 1 && (
        <ParseStep
          preview={preview!}
          mapping={current!}
          config={config!}
          accounts={accounts.data ?? []}
          templates={templates.data ?? []}
          template={template}
          onTemplate={applyTemplate}
          onForget={() => setTemplate("")}
          onChange={(next) => apply(config!, next, overrides)}
          onConfig={(nextConfig, nextMapping) => apply(nextConfig, nextMapping, overrides)}
        />
      )}

      {ready && step === 2 && (
        <AssetsStep
          preview={preview!}
          mapping={current!}
          onChange={(next) => apply(config!, next, overrides)}
        />
      )}

      {ready && step === 3 && (
        <CommitStep
          preview={preview!}
          overrides={overrides}
          onOverrides={(next) => apply(config!, mapping, next)}
          options={options}
          onOptions={setOptions}
          onCommit={() => commit.mutate()}
          pending={commit.isPending}
          error={commit.error}
          result={result}
        />
      )}
    </Page>
  );
}
