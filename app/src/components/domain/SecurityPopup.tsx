import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { usePositionReturn, usePositions, useSecurities } from "../../lib/queries";
import { AlertDialog, NoteDialog } from "./AlertDialogs";
import { WatchlistPicker } from "./WatchlistPicker";
import { useNav } from "../../lib/nav";
import { AssignDialog } from "./AssignDialog";
import { ClassificationPanel } from "./SecurityClassification";
import { Modal, Panel } from "../ui";
import type { AlertInput, PositionRow, SecurityEventInput, SecurityRow, TaxonomyData } from "../../lib/types";
import { useAsOf } from "../../lib/asOf";
import { AlertsPanel } from "./securityCard/AlertsPanel";
import { CardHeader } from "./securityCard/CardHeader";
import { FactsPanel } from "./securityCard/FactsPanel";
import { QuotePanel } from "./securityCard/QuotePanel";
import { RecentTransactions } from "./securityCard/RecentTransactions";

/** Opened from an id alone; a sold-out instrument still shows what is true of it. */

export function SecurityCard({ securityId, onClose }: { securityId: string; onClose: () => void }) {
  const { t } = useLingui();
  const nav = useNav();
  const to = useAsOf().date;

  const positions = usePositions(to);
  const securities = useSecurities();
  const row: PositionRow | undefined = positions.data?.rows.find((r) => r.security_id === securityId);
  const security: SecurityRow | undefined = securities.data?.find((s) => s.id === securityId);
  const currency = positions.data?.base_currency ?? "";
  const symbol = row?.symbol ?? security?.symbol ?? securityId;
  const name = row?.name ?? security?.name ?? "";

  // Return since the first lot; without a position there is no window to measure over.
  const returns = usePositionReturn(securityId, row?.lots[0]?.acquired_at ?? null, to);
  // The split dialog opens over this one; while it stands, Escape belongs to it.
  const [splitting, setSplitting] = useState<TaxonomyData | null>(null);
  // Editors over this card; like the split dialog, each takes Escape while it stands.
  const [alertDraft, setAlertDraft] = useState<AlertInput | null>(null);
  const [note, setNote] = useState<{ draft: SecurityEventInput; reported: boolean } | null>(null);
  const [watching, setWatching] = useState(false);

  // Two queries decide the shape of the card — whether it is held, and what it is. Until both
  // have answered, every block stands in its final size rather than growing into it.
  const loading = positions.isPending || securities.isPending;

  return (
    <Modal
      title={name || symbol}
      onClose={() => {
        if (splitting) setSplitting(null);
        else if (alertDraft) setAlertDraft(null);
        else if (note) setNote(null);
        else if (watching) setWatching(false);
        else onClose();
      }}
      wide
      foot={
        <>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            <Trans>Close</Trans>
          </button>
          <button type="button" className="btn btn--ghost" onClick={() => setWatching(true)}>
            <Trans>Watchlists…</Trans>
          </button>
          <button
            type="button"
            className="btn btn--ghost"
            onClick={() => {
              onClose();
              nav.go("transactions", symbol);
            }}
          >
            <Trans>Show transactions</Trans>
          </button>
          {(loading || security) && (
            <button
              type="button"
              className="btn"
              disabled={loading}
              onClick={() => {
                onClose();
                nav.go("securities", symbol);
              }}
            >
              <Trans>Open in the directory</Trans>
            </button>
          )}
        </>
      }
    >
      {watching && <WatchlistPicker securityId={securityId} onClose={() => setWatching(false)} />}
      <CardHeader
        symbol={symbol}
        name={name}
        security={security}
        row={row}
        currency={currency}
        returns={returns}
        loading={loading}
      />

      <QuotePanel securityId={securityId} symbol={symbol} />

      <div className="grid-2 stack">
        <FactsPanel security={security} row={row} currency={currency} returns={returns} loading={loading} />
        <Panel title={t`Classification`}>
          <ClassificationPanel securityId={securityId} security={security} onSplit={setSplitting} />
        </Panel>
      </div>

      <AlertsPanel securityId={securityId} security={security} onAlert={setAlertDraft} onNote={setNote} />

      {alertDraft && (
        <AlertDialog
          draft={alertDraft}
          securities={security ? [security] : []}
          fixed
          onChange={setAlertDraft}
          onClose={() => setAlertDraft(null)}
        />
      )}
      {note && (
        <NoteDialog
          draft={note.draft}
          reported={note.reported}
          onChange={(next) => setNote({ ...note, draft: next })}
          onClose={() => setNote(null)}
        />
      )}

      {splitting && (
        <AssignDialog
          taxonomy={splitting}
          subjectId={securityId}
          name={name ? `${symbol} · ${name}` : symbol}
          onClose={() => setSplitting(null)}
          onSaved={() => setSplitting(null)}
        />
      )}

      <RecentTransactions securityId={securityId} />
    </Modal>
  );
}
