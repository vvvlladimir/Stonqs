import {
  ArrowRightIcon,
  ArrowsClockwiseIcon,
  CheckCircleIcon,
  DownloadSimpleIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";

import { formatDateTime } from "../../lib/format";
import { useUpdates } from "../../lib/updates";
import { Bar, Markdown, Modal } from "../ui";

/**
 * The one place an update is offered. It shows what the release says about itself and leaves the
 * decision to the user: nothing is downloaded, and nothing is restarted, until a button here.
 *
 * Every stage wears the same shape — a status line naming the two versions, then whatever that
 * stage has to say — so pressing `Update now` changes the sentence and the controls, never the
 * dialog under them.
 */
export function UpdateDialog() {
  const updates = useUpdates();
  const { t } = useLingui();
  if (!updates) return null;

  const { stage, current, update, progress, error } = updates;
  if (stage === "idle" || stage === "checking") return null;

  if (stage === "current")
    return (
      <Modal
        title={t`No update available`}
        onClose={updates.later}
        foot={
          <>
            <span className="spacer" />
            <button className="btn" onClick={updates.later}>
              <Trans>Close</Trans>
            </button>
          </>
        }
      >
        <UpdateNote tone="ok" icon={<CheckCircleIcon weight="fill" />}>
          <Trans>This is the newest published version.</Trans>
        </UpdateNote>
        {current && <VersionLine from={current} />}
      </Modal>
    );

  if (stage === "ready")
    return (
      <Modal
        title={t`Update installed`}
        onClose={updates.later}
        foot={
          <>
            <span className="spacer" />
            <button className="btn btn--ghost" onClick={updates.later}>
              <Trans>Later</Trans>
            </button>
            <button className="btn" onClick={updates.relaunch}>
              <Trans>Restart now</Trans>
            </button>
          </>
        }
      >
        <UpdateNote tone="ok" icon={<CheckCircleIcon weight="fill" />}>
          <Trans>The new version is in place and starts the next time the app opens.</Trans>
        </UpdateNote>
        {update && <VersionLine from={current} to={update.version} />}
      </Modal>
    );

  if (stage === "failed")
    return (
      <Modal
        title={t`Update failed`}
        onClose={updates.later}
        foot={
          <>
            <span className="spacer" />
            <button className="btn btn--ghost" onClick={updates.later}>
              <Trans>Close</Trans>
            </button>
            <button className="btn" onClick={updates.check}>
              <Trans>Try again</Trans>
            </button>
          </>
        }
      >
        <UpdateNote tone="warn" icon={<WarningCircleIcon weight="fill" />}>
          <Trans>
            The update could not be downloaded or installed. Nothing has changed — this version keeps working,
            and the new one can be downloaded by hand from the releases page.
          </Trans>
        </UpdateNote>
        {/* English, from the plugin: a detail for a bug report, not an explanation. */}
        {error && <p className="upd__raw mono dim">{error}</p>}
      </Modal>
    );

  if (!update) return null;

  if (stage === "installing")
    return (
      <Modal title={t`Downloading the update`} onClose={updates.later}>
        <VersionLine from={current} to={update.version} />
        <div className="upd__progress">
          {/* No size means no fraction: the bar says "working", not "this far along". */}
          <Bar share={progress ?? 1} size="lg" />
          <p className="upd__progress-note">
            <span className="muted">
              {progress === null ? (
                <Trans>Downloading…</Trans>
              ) : (
                <Trans>{Math.round(progress * 100)}% downloaded</Trans>
              )}
            </span>
            <span className="dim">
              <Trans>The app restarts once it is in place.</Trans>
            </span>
          </p>
        </div>
      </Modal>
    );

  const notes = releaseNotes(update.notes);

  return (
    <Modal
      title={t`Update available`}
      onClose={updates.later}
      foot={
        <>
          <button className="btn btn--quiet" onClick={updates.skip}>
            <Trans>Skip this version</Trans>
          </button>
          <span className="spacer" />
          <button className="btn btn--ghost" onClick={updates.later}>
            <Trans>Later</Trans>
          </button>
          <button className="btn" onClick={updates.install}>
            <DownloadSimpleIcon weight="bold" />
            <Trans>Update now</Trans>
          </button>
        </>
      }
    >
      <VersionLine from={current} to={update.version} date={update.date} />
      {notes ? (
        <section className="upd__notes">
          <h3 className="upd__notes-head">
            <ArrowsClockwiseIcon weight="bold" />
            <Trans>What changed</Trans>
          </h3>
          <div className="upd__notes-body">
            <Markdown>{notes}</Markdown>
          </div>
        </section>
      ) : (
        <p className="muted">
          <Trans>This release ships no notes of its own.</Trans>
        </p>
      )}
    </Modal>
  );
}

/** The two versions side by side — the fact the whole dialog is about, stated once. */
function VersionLine({ from, to, date }: { from: string | null; to?: string; date?: string | null }) {
  return (
    <div className="upd__vers">
      {from && <span className="upd__ver num">{from}</span>}
      {from && to && <ArrowRightIcon className="upd__arrow" weight="bold" />}
      {to && <span className="upd__ver upd__ver--new num">{to}</span>}
      {date && <span className="upd__when dim">{formatDateTime(date)}</span>}
    </div>
  );
}

function UpdateNote({ tone, icon, children }: { tone: "ok" | "warn"; icon: ReactNode; children: ReactNode }) {
  return (
    <p className={`upd__note upd__note--${tone}`}>
      <span className="upd__note-icon">{icon}</span>
      <span>{children}</span>
    </p>
  );
}

/**
 * The release body cut down to what a reader of this dialog wants. Its own version heading is
 * already the line above, and the commit hash the changelog generator appends to every entry
 * links into the repository rather than describing the change.
 */
function releaseNotes(raw: string): string {
  return raw
    .replace(/^\s*#{1,6}[ \t]+\[?v?\d[^\n]*\n+/, "")
    .replace(/[ \t]*\(\[[0-9a-f]{7,40}\]\([^)]*\)\)/g, "")
    .trim();
}
