import { Trans, useLingui } from "@lingui/react/macro";

import { useUpdates } from "../../lib/updates";
import { Bar, Buttons, Markdown, Modal } from "../ui";

/**
 * The one place an update is offered. It shows what the release says about itself and leaves the
 * decision to the user: nothing is downloaded, and nothing is restarted, until a button here.
 */
export function UpdateDialog() {
  const updates = useUpdates();
  const { t } = useLingui();
  if (!updates) return null;

  const { stage, update, progress, error } = updates;
  if (stage === "idle" || stage === "checking") return null;

  if (stage === "current")
    return (
      <Modal
        title={t`No update available`}
        onClose={updates.later}
        foot={
          <Buttons>
            <button className="btn" onClick={updates.later}>
              <Trans>Close</Trans>
            </button>
          </Buttons>
        }
      >
        <p>
          <Trans>This is the newest published version.</Trans>
        </p>
      </Modal>
    );

  if (stage === "ready")
    return (
      <Modal
        title={t`Update installed`}
        onClose={updates.later}
        foot={
          <Buttons>
            <button className="btn" onClick={updates.relaunch}>
              <Trans>Restart now</Trans>
            </button>
            <button className="btn btn--ghost" onClick={updates.later}>
              <Trans>Later</Trans>
            </button>
          </Buttons>
        }
      >
        <p>
          <Trans>The new version is in place and starts the next time the app opens.</Trans>
        </p>
      </Modal>
    );

  if (stage === "failed")
    return (
      <Modal
        title={t`Update failed`}
        onClose={updates.later}
        foot={
          <Buttons>
            <button className="btn" onClick={updates.check}>
              <Trans>Try again</Trans>
            </button>
            <button className="btn btn--ghost" onClick={updates.later}>
              <Trans>Close</Trans>
            </button>
          </Buttons>
        }
      >
        <p>
          <Trans>
            The update could not be downloaded or installed. Nothing has changed — this version keeps working,
            and the new one can be downloaded by hand from the releases page.
          </Trans>
        </p>
        {/* English, from the plugin: a detail for a bug report, not an explanation. */}
        {error && <p className="dim num">{error}</p>}
      </Modal>
    );

  if (!update) return null;

  if (stage === "installing")
    return (
      <Modal title={t`Downloading the update`} onClose={updates.later}>
        <p>
          <Trans>Version {update.version} is downloading. The app restarts once it is in place.</Trans>
        </p>
        {/* No size means no fraction: the bar says "working", not "this far along". */}
        <Bar fill={progress === null ? "100%" : `${Math.round(progress * 100)}%`} size="lg" />
      </Modal>
    );

  return (
    <Modal
      title={t`Update available`}
      onClose={updates.later}
      foot={
        <Buttons>
          <button className="btn" onClick={updates.install}>
            <Trans>Update now</Trans>
          </button>
          <button className="btn btn--ghost" onClick={updates.later}>
            <Trans>Later</Trans>
          </button>
          <button className="btn btn--ghost" onClick={updates.skip}>
            <Trans>Skip this version</Trans>
          </button>
        </Buttons>
      }
    >
      <p>
        <Trans>Version {update.version} is available.</Trans>
        {update.date && ` · ${update.date}`}
      </p>
      {update.notes && <Markdown>{update.notes}</Markdown>}
    </Modal>
  );
}
