import { Trans, useLingui } from "@lingui/react/macro";
import { ClockCounterClockwiseIcon, PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { Badge, Buttons, ListRow, Money } from "../../components/ui";
import { formatDay } from "../../lib/format";
import { assetKindLabel } from "../../lib/kinds";
import type { Asset, AssetHolding } from "../../lib/types";

/** One thing owned or owed. `holding` is absent while it waits for its first figure. */
export function AssetCard({
  asset,
  holding,
  securedByName,
  onValues,
  onEdit,
  onDelete,
}: {
  asset: Asset;
  holding: AssetHolding | undefined;
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
          <Trans>valued {formatDay(holding.valued_on)}</Trans>
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
        schedule ? (
          <span className="dim">
            <Trans>
              {String(Number(schedule.rate) * 100)}% a year ·{" "}
              <Money value={schedule.monthly_payment} currency={asset.currency} /> a month
              {schedule.ends_on ? <> · ends {formatDay(schedule.ends_on)}</> : null}
            </Trans>
          </span>
        ) : asset.note ? (
          <span className="dim">{asset.note}</span>
        ) : undefined
      }
    />
  );
}
