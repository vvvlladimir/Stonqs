import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Banner, CheckField, ErrorText, InfoHeading, List } from "../../ui";
import { SourceRow } from "./SourceRow";
import { CustomSourceDialog } from "./CustomSourceDialog";
import { OwnSources } from "./OwnSources";
import { useSourceKeys } from "./useSourceKeys";
import type { SourceNeeds } from "./model";
import type { CustomSource, MarketSourceRow } from "../../../lib/types";

/** How the question is left: the set as it stands, every keyless source, or no answer. */
export type Finish = "chosen" | "all" | "later";

/** Why the seal is refused, in one line, on the button that refuses it. */
function useMissing(needs: SourceNeeds): string {
  const { t } = useLingui();
  return !needs.quotes
    ? needs.rates
      ? t`Pick a source of prices first.`
      : t`Pick a source of prices and one of exchange rates first.`
    : t`Pick a source of exchange rates first.`;
}

export function SourcesActions({
  configured,
  needs,
  busy,
  allOn,
  pending,
  error,
  onFinish,
}: {
  configured: boolean;
  needs: SourceNeeds;
  busy: boolean;
  allOn: boolean;
  pending: Finish | null;
  error: Error | null;
  onFinish: (how: Finish) => void;
}) {
  const missing = useMissing(needs);
  return (
    <>
      <button type="button" className="btn btn--quiet" disabled={busy} onClick={() => onFinish("later")}>
        {configured ? <Trans>Close</Trans> : <Trans>Decide later</Trans>}
      </button>
      <span className="spacer" />
      <ErrorText error={error} />
      <button
        type="button"
        className="btn btn--ghost"
        disabled={busy || !needs.ok}
        data-tip={needs.ok ? undefined : missing}
        onClick={() => onFinish("chosen")}
      >
        {pending === "chosen" ? <Trans>Saving…</Trans> : <Trans>Use these sources</Trans>}
      </button>
      <button type="button" className="btn" disabled={busy || allOn} onClick={() => onFinish("all")}>
        {pending === "all" ? <Trans>Saving…</Trans> : <Trans>Select all and continue</Trans>}
      </button>
    </>
  );
}

export interface Adoption {
  orphans: number;
  source: string;
  adopt: boolean;
  setAdopt: (adopt: boolean) => void;
}

export function SourcesBody({
  intro,
  rows,
  mine,
  needs,
  busy,
  onSwitch,
  onRefresh,
  adoption,
}: {
  intro: boolean;
  rows: MarketSourceRow[];
  mine: CustomSource[];
  needs: SourceNeeds;
  busy: boolean;
  onSwitch: (id: string, on: boolean) => void;
  onRefresh: () => void;
  adoption: Adoption | null;
}) {
  const { t } = useLingui();
  const missing = useMissing(needs);
  const keyDialogs = useSourceKeys(rows, onRefresh);
  const [editingCustom, setEditingCustom] = useState<CustomSource | "new" | null>(null);
  // Named as the catalog's message names them.
  const firstQuotes = adoption?.source;
  const orphans = adoption?.orphans;

  return (
    <div className="stack">
      {intro && (
        <p className="dim">
          <Trans>
            Prices, exchange rates and instrument details are fetched from services other people run. Until
            you turn one on the app asks nobody and prices are typed in by hand. Turning a source on means its
            own terms apply to those requests.
          </Trans>
        </p>
      )}

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
            onSwitch={(on) => onSwitch(row.id, on)}
            onKey={() => keyDialogs.open(row.id)}
          />
        ))}
      </List>

      <OwnSources sources={mine} busy={busy} onEdit={setEditingCustom} />

      {adoption && (
        <CheckField
          label={t`Use ${firstQuotes} for the ${orphans} instruments that have no price source`}
          hint={t`They were added while no source was on, so nothing fetches their prices. An instrument you gave a source of its own keeps it.`}
          checked={adoption.adopt}
          onChange={adoption.setAdopt}
        />
      )}

      {editingCustom && (
        <CustomSourceDialog
          source={editingCustom === "new" ? null : editingCustom}
          onKey={keyDialogs.open}
          onClose={() => {
            onRefresh();
            setEditingCustom(null);
          }}
        />
      )}
      {keyDialogs.dialogs}
    </div>
  );
}
