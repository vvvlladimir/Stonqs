import { Trans } from "@lingui/react/macro";
import { Badge } from "../../components/ui";
import type { ImportPreviewData } from "../../lib/types";

/**
 * What the file amounts to right now. It rides in the wizard's footer rather
 * than in a step, because every decision the user makes moves these numbers.
 */
export function Summary({ preview, pending }: { preview: ImportPreviewData; pending: boolean }) {
  const s = preview.summary;
  return (
    <div className="inline">
      {pending && (
        <span className="muted">
          <Trans>Recomputing…</Trans>
        </span>
      )}
      <Badge tone={s.ready > 0 ? "in" : "neutral"}>
        <Trans>ready {s.ready}</Trans>
      </Badge>
      {s.invalid > 0 && (
        <Badge tone="out">
          <Trans>errors {s.invalid}</Trans>
        </Badge>
      )}
      {s.unknown_securities > 0 && (
        <Badge tone="warn">
          <Trans>no instrument {s.unknown_securities}</Trans>
        </Badge>
      )}
      {s.duplicates > 0 && (
        <Badge>
          <Trans>duplicates {s.duplicates}</Trans>
        </Badge>
      )}
      {s.ignored > 0 && (
        <Badge>
          <Trans>skipped {s.ignored}</Trans>
        </Badge>
      )}
      {s.warnings > 0 && (
        <Badge tone="warn">
          <Trans>notices {s.warnings}</Trans>
        </Badge>
      )}
      <span className="dim">
        <Trans>of {s.total}</Trans>
      </span>
    </div>
  );
}
