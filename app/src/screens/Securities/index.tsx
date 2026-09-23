import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { ArrowsClockwiseIcon, DotsThreeIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { useIsWide } from "../../lib/useLayout";
import { affects, useInvalidate, useQuoteProviders, useSecurities } from "../../lib/queries";
import { ListingPicker } from "../../components/domain/ListingPicker";
import { Page } from "../../components/Page";
import { useRefreshStatus } from "../../components/domain/MarketRefresh";
import {
  Banner,
  ErrorText,
  Panel,
  Pending,
  QueryError,
  SearchBox,
  Seg,
  SelectionBar,
  useMenu,
  useSelection,
  type MenuItem,
} from "../../components/ui";
import type { AttributePreview, SecurityRow, SecurityInput } from "../../lib/types";
import { AttributeImportDialog } from "./AttributeImportDialog";
import { SecurityForm } from "./SecurityForm";
import { Splits } from "./Splits";
import { SecurityTable } from "./SecurityTable";
import { cuts, EMPTY, currencyMismatch, inCut, type Cut } from "./model";

export function Securities({ focus }: { focus?: string | null }) {
  const { t, i18n } = useLingui();
  const isWide = useIsWide();
  const invalidate = useInvalidate();
  const securities = useSecurities();
  const providers = useQuoteProviders();
  const [draft, setDraft] = useState<SecurityInput | null>(null);
  const [listingsFor, setListingsFor] = useState<string | null>(null);
  const [splitsFor, setSplitsFor] = useState<string | null>(null);
  const [cut, setCut] = useState<Cut>("all");
  // Navigation hints use the same visible search filter as typed input. The screen is keyed by
  // the hint (`App`), so arriving with a new one starts here rather than syncing in an effect.
  const [query, setQuery] = useState(focus ?? "");
  // The file is kept beside its preview: the commit reads it again, so the plan shown and the
  // plan written are built from the same bytes rather than from what the dialog holds.
  const [attributeFile, setAttributeFile] = useState<{ path: string; preview: AttributePreview } | null>(
    null,
  );
  const [fileError, setFileError] = useState<string | null>(null);
  const menu = useMenu();

  // Build the selection hook before early returns.
  const rows = securities.data ?? [];
  const broken = rows.filter((row) => row.needs_lookup);
  const venueless = rows.filter((row) => row.mic === null);
  // Named in the summary because the symptom sits in one cell of one row otherwise.
  const thin = rows.filter((row) => row.sparse_history);
  // Quote currency may differ from the directory currency; that often indicates another listing.
  const mismatched = rows.filter(currencyMismatch);
  const needle = query.trim().toLowerCase();
  const shown = rows.filter(
    (row) =>
      inCut(row, cut) &&
      (!needle ||
        row.symbol.toLowerCase().includes(needle) ||
        row.name.toLowerCase().includes(needle) ||
        (row.isin ?? "").toLowerCase().includes(needle)),
  );
  const selection = useSelection(shown.map((row) => row.id));

  // Reuse the shared refresh job used by Settings.
  const { running } = useRefreshStatus();
  const refresh = useMutation({ mutationFn: () => api.marketRefresh("catch_up") });

  const save = useMutation({
    mutationFn: api.securitySave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.securities);
    },
  });
  const remove = useMutation({
    mutationFn: api.securityDelete,
    onSuccess: () => invalidate(...affects.securities),
  });

  /** Delete selected securities one by one because the host exposes a single-row command. */
  const removeMany = useMutation({
    mutationFn: async (ids: string[]) => {
      for (const id of ids) await api.securityDelete(id);
    },
    onSettled: () => {
      selection.clear();
      invalidate(...affects.securities);
    },
  });

  const importAttributes = useMutation({
    mutationFn: (path: string) => api.attributesImportCommitPath(path),
    onSuccess: () => {
      setAttributeFile(null);
      invalidate(...affects.securities);
    },
  });

  const pickAttributeFile = async () => {
    setFileError(null);
    const path = await openFile({ multiple: false, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (typeof path !== "string") return;
    try {
      setAttributeFile({ path, preview: await api.attributesImportPreviewPath(path) });
    } catch (error) {
      setFileError(error instanceof Error ? error.message : String(error));
    }
  };

  const exportAttributes = async () => {
    setFileError(null);
    const path = await saveFile({
      defaultPath: "instrument-attributes.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    try {
      await api.attributesExportSave(path);
    } catch (error) {
      setFileError(error instanceof Error ? error.message : String(error));
    }
  };

  /** Resolve imported ISIN placeholders to a provider symbol. */
  const identify = useMutation({
    mutationFn: api.securityIdentify,
    onSuccess: () => invalidate(...affects.securities),
  });

  /** Resolve sequentially to avoid Yahoo rate limits. */
  const identifyAll = async (rows: SecurityRow[]) => {
    for (const row of rows) {
      try {
        await identify.mutateAsync(row.id);
      } catch {
        // One failed lookup must not stop the remaining rows.
      }
    }
  };

  if (securities.isError) return <QueryError error={securities.error} />;
  if (securities.isPending) return <Pending />;

  const picking = rows.find((row) => row.id === listingsFor);
  const splitting = rows.find((row) => row.id === splitsFor);

  const edit = (row: SecurityRow) =>
    setDraft({
      id: row.id,
      symbol: row.symbol,
      name: row.name,
      currency: row.currency,
      kind: row.kind,
      isin: row.isin,
      data_source: row.data_source,
      data_symbol: row.data_symbol,
      quantity_step: row.quantity_step,
      wkn: row.wkn,
      note: row.note,
      attributes: row.attributes,
      other_symbols: row.other_symbols,
    });

  // Keep destructive actions last and visually separated.
  const itemsFor = (row: SecurityRow): MenuItem[] => [
    { label: t`Edit…`, onSelect: () => edit(row) },
    ...(row.needs_lookup
      ? [{ label: t`Identify`, onSelect: () => identify.mutate(row.id), disabled: identify.isPending }]
      : []),
    // Offered for every row: the picker itself says what it needs when the ISIN is missing.
    { label: t`Venues…`, onSelect: () => setListingsFor(row.id) },
    { label: t`Splits…`, onSelect: () => setSplitsFor(row.id) },
    {
      label: t`Delete`,
      danger: true,
      onSelect: () => remove.mutate(row.id),
      disabled: row.transaction_count > 0,
      title: row.transaction_count > 0 ? t`The instrument is used by transactions` : undefined,
    },
  ];

  /** Actions available for the current selection. */
  const locked = shown.filter((row) => selection.has(row.id) && row.transaction_count > 0);

  return (
    <Page
      archetype="registry"
      title={t`Instruments`}
      summary={[
        t`${shown.length} of ${securities.data.length} instruments`,
        venueless.length > 0
          ? plural(venueless.length, { one: "# without a venue", other: "# without a venue" })
          : null,
        broken.length > 0
          ? plural(broken.length, { one: "# not identified", other: "# not identified" })
          : null,
        thin.length > 0
          ? plural(thin.length, { one: "# with too little history", other: "# with too little history" })
          : null,
      ]
        .filter(Boolean)
        .join(" · ")}
      actions={
        <>
          <button className="btn btn--ghost" disabled={running} onClick={() => refresh.mutate()}>
            <ArrowsClockwiseIcon /> {running ? t`Refreshing…` : t`Refresh quotes`}
          </button>
          <button
            type="button"
            className="btn btn--ghost"
            aria-haspopup="menu"
            aria-label={t`More actions`}
            onClick={(e) =>
              menu.openFrom(
                "securities",
                [
                  { label: t`Import attributes…`, onSelect: () => void pickAttributeFile() },
                  { label: t`Export attributes`, onSelect: () => void exportAttributes() },
                ],
                e.currentTarget,
              )
            }
          >
            <DotsThreeIcon />
          </button>
          <button
            className="btn"
            onClick={() => setDraft({ ...EMPTY, data_source: providers.data?.[0] ?? null })}
          >
            <PlusIcon /> <Trans>Add instrument</Trans>
          </button>
        </>
      }
      filters={
        <>
          <SearchBox value={query} onChange={setQuery} placeholder={t`Ticker, name or ISIN`} />
          <Seg options={cuts(i18n)} value={cut} onChange={setCut} label={t`Directory slice`} />
        </>
      }
      banner={
        broken.length + mismatched.length === 0 && fileError === null ? undefined : (
          <>
            {fileError !== null && (
              <Banner tone="bad">
                <Trans>Could not read or write the file: {fileError}</Trans>
              </Banner>
            )}
            {broken.length > 0 && (
              <Banner
                action={
                  <button
                    className="btn btn--sm"
                    disabled={identify.isPending}
                    onClick={() => identifyAll(broken)}
                  >
                    {identify.isPending ? t`Identifying…` : t`Identify all (${broken.length})`}
                  </button>
                }
              >
                <Trans>
                  <b>{broken.length}</b> instruments will get no quotes: their ticker field holds an ISIN —
                  the code of the instrument, not of a listing. The provider will always answer 404.
                </Trans>
              </Banner>
            )}
            {mismatched.length > 0 && (
              <Banner tone="info">
                <Trans>
                  <b>{mismatched.length}</b> instruments are quoted in a currency other than their own. The
                  maths is still right — valuation converts the price at the day's rate — but it usually means
                  the wrong venue is selected: "Venues" in the row shows the other exchanges.
                </Trans>
              </Banner>
            )}
          </>
        )
      }
    >
      <ErrorText error={remove.error} />
      <ErrorText error={identify.error} />
      <ErrorText error={refresh.error} />

      {picking && <ListingPicker row={picking} onClose={() => setListingsFor(null)} />}

      {splitting && <Splits row={splitting} onClose={() => setSplitsFor(null)} />}

      {draft && (
        <SecurityForm
          draft={draft}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      <SelectionBar count={selection.count} onClear={selection.clear}>
        <button
          type="button"
          className="iconbtn iconbtn--sm iconbtn--danger"
          disabled={removeMany.isPending || locked.length > 0}
          title={locked.length > 0 ? t`${locked.length} of the selected are used by transactions` : undefined}
          onClick={() => removeMany.mutate(selection.ids)}
        >
          {removeMany.isPending ? t`Deleting…` : t`Delete (${selection.count})`}
        </button>
      </SelectionBar>
      <ErrorText error={removeMany.error} />

      <Panel table={isWide}>
        <SecurityTable rows={shown} selection={selection} menu={menu} itemsFor={itemsFor} />
      </Panel>
      {attributeFile && (
        <AttributeImportDialog
          preview={attributeFile.preview}
          busy={importAttributes.isPending}
          onClose={() => setAttributeFile(null)}
          onImport={() => importAttributes.mutate(attributeFile.path)}
        />
      )}

      {menu.node}
    </Page>
  );
}
