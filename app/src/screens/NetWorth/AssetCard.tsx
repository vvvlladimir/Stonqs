import { Trans, useLingui } from "@lingui/react/macro";
import { ClockCounterClockwiseIcon, PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { Badge, Buttons, Delta, ListRow, Money } from "../../components/ui";
import { formatDay } from "../../lib/format";
import { assetKindLabel } from "../../lib/kinds";
import type { Asset, AssetHolding } from "../../lib/types";
import { EquityLine, PayoffTrack } from "./AssetFoot";

/** A debt that shrank is good news, a thing owned that shrank is not: the sign alone would
 *  colour both the same way. */
function tone(holding: AssetHolding): "pos" | "neg" | undefined {
  const change = Number(holding.change_base);
  if (!Number.isFinite(change) || change === 0) return undefined;
  const good = holding.side === "OWED" ? change < 0 : change > 0;
  return good ? "pos" : "neg";
}

/** One thing owned or owed. `holding` is absent while it waits for its first figure. */
export function AssetCard({
  asset,
  holding,
  base,
  securedByName,
  onValues,
  onEdit,
  onDelete,
}: {
  asset: Asset;
  holding: AssetHolding | undefined;
  /** The reading's currency; every `_base` figure on the holding is in it. */
  base: string;
  securedByName: string | undefined;
  onValues: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t, i18n } = useLingui();
  const schedule = asset.schedule;

  return (
    <ListRow
      box
      top
      title={asset.name}
      sub={
        <>
          {assetKindLabel(i18n, asset.kind)}
          {securedByName && (
            <>
              {" · "}
              <Trans>secured by {securedByName}</Trans>
            </>
          )}
          {asset.closed_on && (
            <>
              {" "}
              <Badge tone="warn">
                <Trans>closed {formatDay(asset.closed_on)}</Trans>
              </Badge>
            </>
          )}
          {holding?.stale && (
            <>
              {" "}
              {/* Old is not wrong; only the owner can say whether the figure still holds. */}
              <Badge tone="warn">
                <Trans>figure is {holding.days_old} days old</Trans>
              </Badge>
            </>
          )}
        </>
      }
      value={
        holding ? (
          <Money value={holding.amount} currency={holding.currency} />
        ) : (
          <span className="dim">—</span>
        )
      }
      // The day the figure is from, never the reading date: the age of an opinion is the point.
      meta={
        holding ? (
          <>
            <Trans>valued {formatDay(holding.valued_on)}</Trans>
            {holding.change_base !== null && holding.changed_since !== null && (
              <>
                {" "}
                <Delta tone={tone(holding)}>
                  <Money value={holding.change_base} currency={base} signed />
                </Delta>
              </>
            )}
          </>
        ) : (
          <Trans>no figure yet</Trans>
        )
      }
      end={
        <Buttons>
          <button className="btn btn--sm" onClick={onValues} title={t`Value history`}>
            <ClockCounterClockwiseIcon /> <Trans>Update value</Trans>
          </button>
          <button className="iconbtn iconbtn--sm" onClick={onEdit} title={t`Edit`} aria-label={t`Edit`}>
            <PencilSimpleIcon />
          </button>
          <button
            className="iconbtn iconbtn--sm iconbtn--danger"
            onClick={onDelete}
            title={t`Delete`}
            aria-label={t`Delete`}
          >
            <TrashIcon />
          </button>
        </Buttons>
      }
      foot={
        holding?.payoff ? (
          <PayoffTrack asset={asset} holding={holding} />
        ) : holding?.equity_base ? (
          <EquityLine holding={holding} base={base} />
        ) : schedule ? (
          // A debt whose schedule says nothing yet: it has a rate but no figure to apply it to.
          <span className="dim">
            <Trans>
              {String(Number(schedule.rate) * 100)}% a year ·{" "}
              <Money value={schedule.monthly_payment} currency={asset.currency} /> a month
            </Trans>
          </span>
        ) : asset.note ? (
          <span className="dim">{asset.note}</span>
        ) : undefined
      }
    />
  );
}
