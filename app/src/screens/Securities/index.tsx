import { plural } from "@lingui/core/macro";
import { Command } from "../../lib/commands";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ArrowsClockwiseIcon, DotsThreeIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { useIsWide } from "../../lib/useLayout";
import { affects, useInvalidate, useQuoteProviders, useSecurities } from "../../lib/queries";
import { ListingPicker } from "../../components/domain/ListingPicker";
import { Page } from "../../components/Page";
import { useRefreshStatus } from "../../components/domain/MarketRefresh";
import {
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
import type { SecurityRow, SecurityInput } from "../../lib/types";
import { AttributeImportDialog } from "./AttributeImportDialog";
import { SecurityForm } from "./SecurityForm";
import { Splits } from "./Splits";
import { SecurityTable } from "./SecurityTable";
import { Banners } from "./Banners";
import { cuts, EMPTY, currencyMismatch, inCut, matching, toInput, type Cut } from "./model";
import { useAttributeFile } from "./useAttributeFile";
import { useIdentify } from "./useIdentify";

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
  const attributes = useAttributeFile();
  const { identify, identifyAll } = useIdentify();
  const menu = useMenu();

  // Build the selection hook before early returns.
  const rows = securities.data ?? [];
  const broken = rows.filter((row) => row.needs_lookup);
  // Quote currency may differ from the directory currency; that often indicates another listing.
  const mismatched = rows.filter(currencyMismatch);
  const shown = rows.filter((row) => inCut(row, cut) && matching(row, query));
  const selection = useSelection(shown.map((row) => row.id));
  const summary = useDirectorySummary(shown, rows);

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

  if (securities.isError) return <QueryError error={securities.error} />;
  if (securities.isPending) return <Pending />;

  const picking = rows.find((row) => row.id === listingsFor);
  const splitting = rows.find((row) => row.id === splitsFor);

  const edit = (row: SecurityRow) => setDraft(toInput(row));
  const create = () => setDraft({ ...EMPTY, data_source: providers.data?.[0] ?? null });

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
      summary={summary}
      actions={
        <DirectoryActions
          refreshing={running}
          onRefresh={() => refresh.mutate()}
          menu={menu}
          attributes={attributes}
          onCreate={create}
        />
      }
      filters={
        <>
          <SearchBox value={query} onChange={setQuery} placeholder={t`Ticker, name or ISIN`} />
          <Seg options={cuts(i18n)} value={cut} onChange={setCut} label={t`Directory slice`} />
        </>
      }
      banner={
        broken.length + mismatched.length === 0 && attributes.error === null ? undefined : (
          <Banners
            broken={broken}
            mismatched={mismatched}
            fileError={attributes.error}
            identifying={identify.isPending}
            onIdentifyAll={() => identifyAll(broken)}
          />
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
      {attributes.file && (
        <AttributeImportDialog
          preview={attributes.file.preview}
          busy={attributes.importing}
          onClose={attributes.close}
          onImport={attributes.importFile}
        />
      )}

      {menu.node}
    </Page>
  );
}

/** How many are shown, and the three reasons an instrument may be priced wrong — named here
 *  because otherwise each one is a single cell of a single row. */
function useDirectorySummary(shown: SecurityRow[], rows: SecurityRow[]): string {
  const { t } = useLingui();
  const venueless = rows.filter((row) => row.mic === null);
  const broken = rows.filter((row) => row.needs_lookup);
  const thin = rows.filter((row) => row.sparse_history);
  return [
    t`${shown.length} of ${rows.length} instruments`,
    venueless.length > 0
      ? plural(venueless.length, { one: "# without a venue", other: "# without a venue" })
      : null,
    broken.length > 0 ? plural(broken.length, { one: "# not identified", other: "# not identified" }) : null,
    thin.length > 0
      ? plural(thin.length, { one: "# with too little history", other: "# with too little history" })
      : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

function DirectoryActions({
  refreshing,
  onRefresh,
  menu,
  attributes,
  onCreate,
}: {
  refreshing: boolean;
  onRefresh: () => void;
  menu: ReturnType<typeof useMenu>;
  attributes: ReturnType<typeof useAttributeFile>;
  onCreate: () => void;
}) {
  const { t } = useLingui();
  return (
    <>
      <button className="btn btn--ghost" disabled={refreshing} onClick={onRefresh}>
        <ArrowsClockwiseIcon /> {refreshing ? t`Refreshing…` : t`Refresh quotes`}
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
              { label: t`Import attributes…`, onSelect: () => void attributes.pick() },
              { label: t`Export attributes`, onSelect: () => void attributes.exportAll() },
            ],
            e.currentTarget,
          )
        }
      >
        <DotsThreeIcon />
      </button>
      <button className="btn" onClick={onCreate}>
        <PlusIcon /> <Trans>Add instrument</Trans>
      </button>
      <Command id="new" label={t`Add instrument`} run={onCreate} />
      <Command id="newInstrument" run={onCreate} />
    </>
  );
}
