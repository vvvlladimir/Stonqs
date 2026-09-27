import { Trans, useLingui } from "@lingui/react/macro";
import { useAlerts, useSecurityEvents } from "../../../lib/queries";
import { alertToInput, newAlert, newNote, noteToInput } from "../../../lib/alerts";
import { AlertList, EventList } from "../SecurityAlerts";
import { Async, Buttons, Panel, SkeletonRows } from "../../ui";
import type { AlertInput, SecurityEventInput, SecurityRow } from "../../../lib/types";
import { RECENT } from "./recent";

/** Price alerts, then notes, dividends and splits. */
export function AlertsPanel({
  securityId,
  security,
  onAlert,
  onNote,
}: {
  securityId: string;
  security: SecurityRow | undefined;
  onAlert: (draft: AlertInput) => void;
  onNote: (note: { draft: SecurityEventInput; reported: boolean }) => void;
}) {
  const { t } = useLingui();
  const alerts = useAlerts(securityId);
  const events = useSecurityEvents(securityId);
  return (
    <Panel
      title={t`Alerts and events`}
      tools={
        <Buttons>
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            onClick={() => onNote({ draft: newNote(securityId), reported: false })}
          >
            <Trans>Add note</Trans>
          </button>
          <button
            type="button"
            className="btn btn--sm"
            disabled={!security}
            onClick={() => security && onAlert(newAlert(security))}
          >
            <Trans>New alert</Trans>
          </button>
        </Buttons>
      }
    >
      {alerts.data && alerts.data.length > 0 && (
        <AlertList rows={alerts.data} onEdit={(r) => onAlert(alertToInput(r.alert))} />
      )}
      <Async
        query={events}
        pending={<SkeletonRows rows={2} />}
        empty={
          <p className="muted">
            <Trans>No notes, dividends or splits for this instrument.</Trans>
          </p>
        }
      >
        {(list) => (
          <EventList
            rows={list.slice(0, RECENT)}
            onEditNote={(e) => onNote({ draft: noteToInput(e), reported: e.kind !== "NOTE" })}
          />
        )}
      </Async>
    </Panel>
  );
}
