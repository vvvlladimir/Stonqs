import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { CloudArrowDownIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../../lib/api";
import {
  affects,
  keys,
  useInvalidate,
  useMarketCustom,
  useMarketSources,
  useSecurities,
  useSettings,
} from "../../../lib/queries";
import {
  CustomSourceDialog,
  SourceRow,
  sourceNeeds,
  useSourceKeys,
} from "../../../components/domain/sources";
import type { CustomSource } from "../../../lib/types";
import { Banner, Empty, ListRow, Panel } from "../../../components/ui";

/** Every source the app can ask, which of them it does, and the keys that open the rest. */
export function SourcesPanel() {
  const { t } = useLingui();
  const sources = useMarketSources();
  const custom = useMarketCustom();
  const settings = useSettings();
  const securities = useSecurities();
  const invalidate = useInvalidate();
  const [editingCustom, setEditingCustom] = useState<CustomSource | "new" | null>(null);

  const rows = sources.data ?? [];
  // Until the question is answered every source is off whatever its switch says, so the rows
  // show the picks rather than a table of "Off" nobody chose.
  const configured = settings.data?.sources_configured ?? false;

  // `keys.settings()` with them: turning a source on is itself the answer to "where may data
  // come from", so what the rest of the app reads off `sources_configured` changes with it.
  const refresh = () =>
    invalidate(keys.settings(), keys.marketSources(), keys.marketCustom(), keys.quoteProviders());
  const keyDialogs = useSourceKeys(rows, refresh);
  const switchOne = useMutation({
    mutationFn: ({ id, on }: { id: string; on: boolean }) => api.marketSourceSwitch(id, on),
    onSuccess: refresh,
  });

  // The rows below *are* the picker: `Turn on` is the answer, and the notice says what the
  // answer still lacks rather than asking for a second press to apply it (ADR-0076). It is
  // about what cannot be priced, not about the switches — a set that is merely small is fine.
  // Read through the same gate the host applies: while the question has never been answered
  // every switch is off whatever it says, so a default-on rate source is not an answer to it.
  const picked = sourceNeeds(rows, custom.data ?? [], settings.data?.market_sources ?? {});
  const needs = configured ? picked : { quotes: false, rates: false, ok: false };

  // An instrument added while nothing was on is priced by hand and no refresh would notice it.
  const orphans = (securities.data ?? []).filter((s) => s.data_source === null).length;
  const firstQuotes = rows.find((row) => row.active && row.capabilities.includes("quotes"))?.id;
  const adopt = useMutation({
    mutationFn: (source: string) => api.securitiesAdoptSource(source),
    onSuccess: () => invalidate(keys.marketSources(), ...affects.securities),
  });

  return (
    <>
      <Panel
        title={t`Data sources`}
        info={t`Where quotes and exchange rates come from; an instrument's own source is asked first.`}
      >
        {!needs.ok && (
          <Banner tone="bad">
            {!needs.quotes && !needs.rates ? (
              <Trans>
                Nothing is fetched: no source of prices and none of exchange rates is on, so both are whatever
                was typed in by hand. Turn on at least one of each below.
              </Trans>
            ) : !needs.quotes ? (
              <Trans>
                No source of prices is on, so quotes are whatever was typed in by hand. Turn one on below.
              </Trans>
            ) : (
              <Trans>
                No source of exchange rates is on, so anything held in a currency other than the
                portfolio&apos;s cannot be valued. Turn one on below.
              </Trans>
            )}
          </Banner>
        )}
        {orphans > 0 && firstQuotes && (
          <Banner
            action={
              <button
                type="button"
                className="btn btn--sm"
                disabled={adopt.isPending}
                onClick={() => adopt.mutate(firstQuotes)}
              >
                {adopt.isPending ? <Trans>Saving…</Trans> : <Trans>Use {firstQuotes}</Trans>}
              </button>
            }
          >
            <Plural
              value={orphans}
              one="# instrument has no price source: it was added while nothing was on, so nothing fetches its prices."
              few="# instruments have no price source: they were added while nothing was on, so nothing fetches their prices."
              many="# instruments have no price source: they were added while nothing was on, so nothing fetches their prices."
              other="# instruments have no price source: they were added while nothing was on, so nothing fetches their prices."
            />{" "}
            <Trans>An instrument you gave a source of its own keeps it.</Trans>
          </Banner>
        )}
        {rows.map((row) => (
          <SourceRow
            key={row.id}
            row={row}
            pending={!configured}
            busy={switchOne.isPending}
            onSwitch={(on) => switchOne.mutate({ id: row.id, on })}
            onKey={() => keyDialogs.open(row.id)}
          />
        ))}
      </Panel>

      <Panel
        title={t`Your own sources`}
        info={t`A price feed you describe: its address and where the dates and closes are in the answer.`}
        tools={
          <button type="button" className="btn btn--sm btn--ghost" onClick={() => setEditingCustom("new")}>
            <PlusIcon /> <Trans>Add source</Trans>
          </button>
        }
      >
        {(custom.data ?? []).length === 0 ? (
          <Empty title={t`No sources of your own`}>
            <Trans>Any JSON or CSV price endpoint can be added without an update of the app.</Trans>
          </Empty>
        ) : (
          (custom.data ?? []).map((source) => (
            <ListRow
              key={source.id}
              box
              lead={<CloudArrowDownIcon />}
              title={source.label}
              sub={`${source.role === "fx" ? t`exchange rates` : t`quotes`} · ${source.url}`}
              end={
                <button
                  type="button"
                  className="btn btn--sm btn--ghost"
                  onClick={() => setEditingCustom(source)}
                >
                  <Trans>Edit</Trans>
                </button>
              }
            />
          ))
        )}
      </Panel>

      {keyDialogs.dialogs}
      {editingCustom && (
        <CustomSourceDialog
          source={editingCustom === "new" ? null : editingCustom}
          onKey={keyDialogs.open}
          onClose={() => {
            refresh();
            setEditingCustom(null);
          }}
        />
      )}
    </>
  );
}
