import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useEffect, useState } from "react";
import { FlaskIcon, NotePencilIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { alertToInput, newAlert, newNote, noteToInput } from "../../lib/alerts";
import {
  keys,
  useAlertCrossings,
  useAlerts,
  useInvalidate,
  useSecurities,
  useSecurityEvents,
} from "../../lib/queries";
import { Page } from "../../components/Page";
import { AlertDialog, NoteDialog } from "../../components/domain/AlertDialogs";
import { AlertList, CrossingLog, EventList } from "../../components/domain/SecurityAlerts";
import { Async, Empty, Modal, Panel, Pending, QueryError } from "../../components/ui";
import type { AlertInput, SecurityEventInput } from "../../lib/types";
import { DevSimulator } from "./DevSimulator";

/** Log lines and events listed at once; older ones stay on each instrument's card. */
const SHOWN = 100;

export function Alerts() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const alerts = useAlerts();
  const crossings = useAlertCrossings(SHOWN);
  const events = useSecurityEvents();
  const securities = useSecurities();
  const [draft, setDraft] = useState<AlertInput | null>(null);
  const [note, setNote] = useState<{ draft: SecurityEventInput; reported: boolean } | null>(null);
  const [simulating, setSimulating] = useState(false);
  const [logOpen, setLogOpen] = useState(false);

  // Opening the log is looking at it. Only the navigation dot is refreshed: the lines in the popup
  // keep their dot until the log is next read, so what was new stays visible while it is open.
  const logVersion = crossings.dataUpdatedAt;
  useEffect(() => {
    if (!logOpen || logVersion === 0) return;
    void api.alertsMarkSeen().then(() => invalidate(keys.alertsUnseen()));
  }, [logOpen, logVersion, invalidate]);

  if (alerts.isError) return <QueryError error={alerts.error} />;
  if (!alerts.data || !securities.data) return <Pending />;

  const rows = alerts.data;
  const unseen = crossings.data?.filter((c) => !c.seen).length ?? 0;
  const first = securities.data[0];

  return (
    <Page
      archetype="registry"
      title={t`Alerts`}
      summary={[
        plural(rows.length, { one: "# rule", other: "# rules" }),
        plural(unseen, { one: "# new crossing", other: "# new crossings" }),
      ].join(" · ")}
      actions={
        <>
          {import.meta.env.DEV && (
            <button
              className="btn btn--ghost"
              disabled={rows.length === 0}
              onClick={() => setSimulating(true)}
            >
              <FlaskIcon /> <Trans>Simulate</Trans>
            </button>
          )}
          <button
            className="btn btn--ghost"
            disabled={!first}
            onClick={() => first && setNote({ draft: newNote(first.id), reported: false })}
          >
            <NotePencilIcon /> <Trans>Add note</Trans>
          </button>
          <button className="btn" disabled={!first} onClick={() => first && setDraft(newAlert(first))}>
            <PlusIcon /> <Trans>New alert</Trans>
          </button>
        </>
      }
    >
      {draft && (
        <AlertDialog
          draft={draft}
          securities={securities.data}
          fixed={Boolean(draft.id)}
          onChange={setDraft}
          onClose={() => setDraft(null)}
        />
      )}
      {note && (
        <NoteDialog
          draft={note.draft}
          reported={note.reported}
          securities={note.draft.id ? undefined : securities.data}
          onChange={(next) => setNote({ ...note, draft: next })}
          onClose={() => setNote(null)}
        />
      )}
      {simulating && <DevSimulator rows={rows} onClose={() => setSimulating(false)} />}
      {logOpen && (
        <Modal title={t`Log`} onClose={() => setLogOpen(false)}>
          <p className="panel__note">
            <Trans>When a close crossed a level</Trans>
          </p>
          <Async
            query={crossings}
            empty={
              <p className="muted">
                <Trans>Nothing has crossed yet. The log fills after each quote refresh.</Trans>
              </p>
            }
          >
            {(list) => <CrossingLog rows={list} />}
          </Async>
        </Modal>
      )}

      <Panel
        title={t`Rules`}
        tools={
          <button type="button" className="btn btn--sm btn--ghost" onClick={() => setLogOpen(true)}>
            <Trans>Log</Trans>
            {unseen > 0 && <i className="tab-dot" aria-label={t`new crossings to look at`} />}
          </button>
        }
      >
        {rows.length === 0 ? (
          <Empty title={t`No alerts yet`}>
            <Trans>
              An alert is a price level or a day on one instrument. A close that crosses the level in the
              chosen direction is logged below and announced once.
            </Trans>
          </Empty>
        ) : (
          <AlertList rows={rows} named onEdit={(row) => setDraft(alertToInput(row.alert))} />
        )}
      </Panel>

      <Panel
        title={t`Events`}
        info={t`Your own notes, plus the dividends and splits reported by the quote source.`}
      >
        <Async
          query={events}
          empty={
            <p className="muted">
              <Trans>No events yet. Dividends and splits arrive with the next quote refresh.</Trans>
            </p>
          }
        >
          {(list) => (
            <EventList
              rows={list.slice(0, SHOWN)}
              named
              onEditNote={(event) => setNote({ draft: noteToInput(event), reported: event.kind !== "NOTE" })}
            />
          )}
        </Async>
      </Panel>
    </Page>
  );
}
