import { Trans } from "@lingui/react/macro";
import { formatMoney, formatPercent } from "../../lib/format";
import type { AllocationBucket } from "../../lib/types";

/** Persistent hierarchy trail shared by treemap and sunburst charts. */
export function Trail({
  path,
  currency,
}: {
  path: AllocationBucket[];
  currency: string;
  /** Optional full security name for the final breadcrumb. */
  nameOf?: (key: string) => string | undefined;
}) {
  return (
    <div className="trail">
      {path.map((bucket, i) => (
        <span
          key={bucket.key}
          className={`trail__crumb${i === path.length - 1 ? " trail__crumb--last" : ""}`}
        >
          <b>{i === path.length - 1 ? fullLabel(bucket.label) : bucket.label}</b>
          <span className="num">
            {formatMoney(bucket.value_base, currency, { digits: 0 })} (
            {formatPercent(bucket.weight, { digits: 2 })})
          </span>
        </span>
      ))}
      {path.length === 0 && (
        <span className="muted">
          <Trans>Hover a share — its path appears here.</Trans>
        </span>
      )}
    </div>
  );
}

/** Places the ticker before the optional full security name. */
export function fullLabel(short: string, full?: string): string {
  return full && full !== short ? `${short} — ${full}` : short;
}
