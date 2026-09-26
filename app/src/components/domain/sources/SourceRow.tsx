import { Trans, useLingui } from "@lingui/react/macro";
import { msg } from "@lingui/core/macro";
import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { Badge, ListRow } from "../../ui";
import type { MarketSourceRow } from "../../../lib/types";

const CAPABILITY = {
  quotes: msg`quotes`,
  search: msg`search`,
  listings: msg`venues`,
  fx_rates: msg`exchange rates`,
  price_index: msg`inflation`,
} as const;

/**
 * One shipped source: what it answers, where its requests go, and the two decisions about it.
 *
 * `active` is whether it is really asked, and `wanted` is only what was picked — before the
 * sources are confirmed nothing is active, so `pending` is what the setup dialog passes to keep
 * a source it just switched on from reading as "off".
 */
export function SourceRow({
  row,
  busy,
  pending,
  onSwitch,
  onKey,
}: {
  row: MarketSourceRow;
  busy: boolean;
  /** Show the switch's own state rather than whether the source is being asked yet. */
  pending?: boolean;
  onSwitch: (on: boolean) => void;
  onKey: () => void;
}) {
  const { t, i18n } = useLingui();
  const roles = row.capabilities.map((c) => i18n._(CAPABILITY[c])).join(", ");
  const on = pending ? row.wanted : row.active;
  const needsKey = row.wanted && !row.has_key && row.key === "required";

  return (
    <ListRow
      box
      title={row.id}
      sub={
        <>
          {roles} ·{" "}
          <a href={row.site} target="_blank" rel="noreferrer noopener" className="inline">
            {hostOf(row.site)} <ArrowSquareOutIcon />
          </a>
        </>
      }
      end={
        <>
          <Badge tone={on && !needsKey ? "in" : needsKey ? "warn" : "neutral"}>
            {needsKey ? t`Needs a key` : on ? t`On` : t`Off`}
          </Badge>
          {row.key !== "none" && (
            <button type="button" className="btn btn--sm btn--ghost" onClick={onKey}>
              {row.has_key ? <Trans>Replace key</Trans> : <Trans>Add key</Trans>}
            </button>
          )}
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            disabled={busy}
            onClick={() => onSwitch(!row.wanted)}
          >
            {row.wanted ? <Trans>Turn off</Trans> : <Trans>Turn on</Trans>}
          </button>
        </>
      }
    />
  );
}

/** The address without its scheme: the row says where requests go, not how they are addressed. */
function hostOf(site: string): string {
  return site.replace(/^https?:\/\//, "");
}
