import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
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
import { Banner, CheckField, ErrorText, InfoHeading, List, ListRow, Modal } from "../../ui";
import { SourceRow } from "./SourceRow";
import { CustomSourceDialog } from "./CustomSourceDialog";
import { sourceNeeds } from "./model";
import { useSourceKeys } from "./useSourceKeys";
import type { CustomSource, MarketSourceRow } from "../../../lib/types";

/**
 * Where the app may fetch data from, asked once and changeable forever after in Settings.
 *
 * Nothing is on until this is answered (ADR-0076), so the rows show what was *picked* rather
 * than what is being asked: sealing the answer is what turns the picks into requests. A set
 * that cannot price a portfolio is not sealed at all — `Decide later` is the way out, and it
 * leaves the app saying so rather than quietly fetching nothing.
 */
export function SourcesSetup({ onClose }: { onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const sources = useMarketSources();
  const custom = useMarketCustom();
  const settings = useSettings();
  const securities = useSecurities();
  const rows = sources.data ?? [];
  const mine = custom.data ?? [];
  const configured = settings.data?.sources_configured ?? false;
  const [editingCustom, setEditingCustom] = useState<CustomSource | "new" | null>(null);

  // A switch is itself the answer (ADR-0076), so the settings change with the rows.
  const refresh = () =>
    invalidate(keys.settings(), keys.marketSources(), keys.marketCustom(), keys.quoteProviders());
  const keyDialogs = useSourceKeys(rows, refresh);

  const switchOne = useMutation({
    mutationFn: ({ id, on }: { id: string; on: boolean }) => api.marketSourceSwitch(id, on),
    onSuccess: refresh,
  });

  // Everything that can answer without one: a source whose key is required and missing would be
  // switched on into the state that reads as broken.
  const available = rows.filter((row) => row.key !== "required" || row.has_key);
  const allOn = available.length > 0 && available.every((row) => row.wanted);
  const needs = sourceNeeds(rows, mine, settings.data?.market_sources ?? {});

  // An instrument created while no source was on is priced by hand and no refresh would ever
  // notice it, so the first source that arrives is offered to them all at once.
  const orphans = (securities.data ?? []).filter((s) => s.data_source === null).length;
  const firstQuotes = rows.find((row) => row.wanted && quotes(row) && row.key !== "required")?.id;
  const [adopt, setAdopt] = useState(true);

  const seal = async () => {
    await api.marketSourcesConfirm();
    if (adopt && orphans > 0 && firstQuotes) await api.securitiesAdoptSource(firstQuotes);
  };
  const finished = () => {
    invalidate(keys.settings(), keys.marketSources(), keys.quoteProviders(), ...affects.securities);
    onClose();
  };

  const done = useMutation({ mutationFn: seal, onSuccess: finished });
  // One press for the whole question: everything that answers without a key of its own, then
  // sealed. The switches are written one at a time, which is what the picker itself does.
  const selectAll = useMutation({
    mutationFn: async () => {
      for (const row of available.filter((row) => !row.wanted)) {
        await api.marketSourceSwitch(row.id, true);
      }
      await seal();
    },
    onSuccess: finished,
  });

  const busy = done.isPending || selectAll.isPending || switchOne.isPending;
  // Why the seal is refused, in one line, on the button that refuses it.
  const missing = !needs.quotes
    ? needs.rates
      ? t`Pick a source of prices first.`
      : t`Pick a source of prices and one of exchange rates first.`
    : t`Pick a source of exchange rates first.`;

  return (
    <Modal
      wide
      title={<Trans>Where data comes from</Trans>}
      onClose={onClose}
      foot={
        <>
          <button type="button" className="btn btn--quiet" disabled={busy} onClick={onClose}>
            {configured ? <Trans>Close</Trans> : <Trans>Decide later</Trans>}
          </button>
          <span className="spacer" />
          <ErrorText error={done.error ?? selectAll.error ?? switchOne.error} />
          <button
            type="button"
            className="btn btn--ghost"
            disabled={busy || !needs.ok}
            data-tip={needs.ok ? undefined : missing}
            onClick={() => done.mutate()}
          >
            {done.isPending ? <Trans>Saving…</Trans> : <Trans>Use these sources</Trans>}
          </button>
          <button type="button" className="btn" disabled={busy || allOn} onClick={() => selectAll.mutate()}>
            {selectAll.isPending ? <Trans>Saving…</Trans> : <Trans>Select all and continue</Trans>}
          </button>
        </>
      }
    >
      <div className="stack">
        <p className="dim">
          <Trans>
            Prices, exchange rates and instrument details are fetched from services other people run. Until
            you turn one on the app asks nobody and prices are typed in by hand. Turning a source on means its
            own terms apply to those requests.
          </Trans>
        </p>

        {!needs.ok && (
          <Banner>
            {missing}{" "}
            <Trans>
              Without both, holdings in another currency cannot be valued at all. Leaving with Decide later is
              an answer too: prices are then typed in by hand.
            </Trans>
          </Banner>
        )}

        <InfoHeading
          title={t`Shipped sources`}
          info={t`Everything this build can ask. A source with a key of its own answers only once the key is saved.`}
        />
        <List variant="cards">
          {rows.map((row) => (
            <SourceRow
              key={row.id}
              row={row}
              pending
              busy={busy}
              onSwitch={(on) => switchOne.mutate({ id: row.id, on })}
              onKey={() => keyDialogs.open(row.id)}
            />
          ))}
        </List>

        <div className="inline">
          <InfoHeading
            title={t`Your own sources`}
            info={t`Any JSON or CSV price endpoint: its address, and where the dates and closes sit in the answer.`}
          />
          <span className="spacer" />
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            disabled={busy}
            onClick={() => setEditingCustom("new")}
          >
            <PlusIcon /> <Trans>Add source</Trans>
          </button>
        </div>
        {mine.length > 0 && (
          <List variant="cards">
            {mine.map((source) => (
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
            ))}
          </List>
        )}

        {orphans > 0 && firstQuotes && (
          <CheckField
            label={t`Use ${firstQuotes} for the ${orphans} instruments that have no price source`}
            hint={t`They were added while no source was on, so nothing fetches their prices. An instrument you gave a source of its own keeps it.`}
            checked={adopt}
            onChange={setAdopt}
          />
        )}
      </div>

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
      {keyDialogs.dialogs}
    </Modal>
  );
}

/** Whether the source can answer with prices at all — a rate or venue source cannot. */
function quotes(row: MarketSourceRow): boolean {
  return row.capabilities.includes("quotes");
}
