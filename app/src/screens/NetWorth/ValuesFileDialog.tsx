import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Banner, Fact, Facts, Modal } from "../../components/ui";
import type { ValuesPreview } from "../../lib/types";

/** What a valuations file would write, before anything is written. */
export function ValuesFileDialog({
  preview,
  onWrite,
  onClose,
  writing,
}: {
  preview: ValuesPreview;
  onWrite: () => void;
  onClose: () => void;
  writing: boolean;
}) {
  const { t } = useLingui();
  const writes = preview.rows.filter((row) => row.asset_id !== null);
  const replacing = writes.filter((row) => row.replaces).length;
  const things = new Set(writes.map((row) => row.asset_id)).size;
  // Errors only: a warning leaves its row importable, and listing it as left out would be a lie.
  const skipped = [
    ...new Set(preview.problems.filter((p) => p.row !== null && p.severity === "ERROR").map((p) => p.row!)),
  ].sort((a, b) => a - b);
  const repeated = preview.problems
    .filter((p) => p.code === "DUPLICATE_IN_FILE" && p.row !== null)
    .map((p) => p.row!);

  return (
    <Modal
      title={t`Import valuations`}
      onClose={onClose}
      foot={
        <>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            <Trans>Cancel</Trans>
          </button>
          <button type="button" className="btn" disabled={writes.length === 0 || writing} onClick={onWrite}>
            {writing ? <Trans>Writing…</Trans> : <Trans>Write these figures</Trans>}
          </button>
        </>
      }
    >
      <Facts>
        <Fact
          label={<Trans>Figures to write</Trans>}
          value={plural(writes.length, { one: "# figure", other: "# figures" })}
        />
        <Fact label={<Trans>Things they belong to</Trans>} value={String(things)} />
        <Fact
          label={<Trans>Days already answered</Trans>}
          value={plural(replacing, { one: "# figure replaced", other: "# figures replaced" })}
        />
      </Facts>

      {preview.unmatched.length > 0 && (
        // Nothing is created from a file: a file cannot say what kind of thing a name is, nor
        // which way its amount points.
        <Banner tone="warn">
          <Trans>
            No thing here goes by {preview.unmatched.join(", ")} — add it first, then import again.
          </Trans>
        </Banner>
      )}

      {preview.ambiguous.length > 0 && (
        // Two things under one name: the file names neither, so no figure is written for either.
        <Banner tone="warn">
          <Trans>
            More than one thing goes by {preview.ambiguous.join(", ")} — rename one of them, or the file
            cannot say which it means.
          </Trans>
        </Banner>
      )}

      {repeated.length > 0 && (
        <Banner tone="warn">
          <Trans>
            Rows {repeated.slice(0, 8).join(", ")} answer a day the file has already answered — of each such
            day, the last figure is the one kept.
          </Trans>
        </Banner>
      )}

      {skipped.length > 0 && (
        // The rows are named by number rather than explained: the file is the user's own, and a
        // line number is what finds the cell that could not be read.
        <Banner tone="warn">
          <Trans>Rows {skipped.slice(0, 8).join(", ")} could not be read and are left out.</Trans>
        </Banner>
      )}
    </Modal>
  );
}
