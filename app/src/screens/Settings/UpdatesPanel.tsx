import { Trans, useLingui } from "@lingui/react/macro";

import { useUiState } from "../../lib/uiState";
import { useUpdates } from "../../lib/updates";
import { Buttons, CheckField, Form, Panel } from "../../components/ui";

/**
 * Which version is running and whether the app may look for a newer one. The offer itself is a
 * dialog, so this panel never shows release notes: pressing `Check for updates` opens the same
 * one the automatic check does.
 */
export function UpdatesPanel() {
  const { t } = useLingui();
  const { ui, save } = useUiState();
  const updates = useUpdates();

  const skipped = ui.updates.skip;

  return (
    <Panel
      title={t`Updates`}
      info={t`Where this app looks for a newer version, and whether it may look on its own.`}
      note={updates?.current ?? undefined}
    >
      <Form>
        <CheckField
          label={t`Check for updates automatically`}
          hint={t`Once a day, over the internet. Nothing is downloaded before you say so.`}
          checked={ui.updates.auto}
          onChange={(auto) => save((ui) => ({ ...ui, updates: { ...ui.updates, auto } }))}
        />

        {skipped && (
          <p className="dim">
            <Trans>Version {skipped} was skipped and is not offered again.</Trans>{" "}
            <button
              type="button"
              className="btn btn--ghost"
              onClick={() => save((ui) => ({ ...ui, updates: { ...ui.updates, skip: null } }))}
            >
              <Trans>Offer it again</Trans>
            </button>
          </p>
        )}

        <Buttons>
          <button
            type="button"
            className="btn"
            disabled={!updates || updates.stage === "checking"}
            onClick={() => updates?.check()}
          >
            {updates?.asked ? <Trans>Checking…</Trans> : <Trans>Check for updates</Trans>}
          </button>
        </Buttons>
      </Form>
    </Panel>
  );
}
