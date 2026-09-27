import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import { formatDay, formatPercent, toNumber } from "../../lib/format";
import { ListRow, Money, Progress } from "../../components/ui";
import type { LimitUsage } from "../../lib/types";

export function LimitCard({
  usage,
  account,
  onEdit,
  onDelete,
}: {
  usage: LimitUsage;
  account: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const over = (toNumber(usage.share) ?? 0) > 1;

  return (
    <ListRow
      box
      top
      title={usage.name}
      sub={account}
      value={<Money value={usage.used} currency={usage.currency} />}
      meta={
        <Trans>
          of <Money value={usage.allowance} currency={usage.currency} />
        </Trans>
      }
      end={
        <>
          <button type="button" className="iconbtn iconbtn--sm" aria-label={t`Edit`} onClick={onEdit}>
            <PencilSimpleIcon />
          </button>
          <button
            type="button"
            className="iconbtn iconbtn--sm iconbtn--danger"
            aria-label={t`Delete`}
            onClick={onDelete}
          >
            <TrashIcon />
          </button>
        </>
      }
      foot={
        <Progress
          size="sm"
          share={usage.share}
          barTone={over ? "neg" : undefined}
          legend={{
            left: (
              <>
                {formatPercent(usage.share)}
                {" · "}
                <Trans>
                  <Money value={usage.remaining} currency={usage.currency} /> left
                </Trans>
              </>
            ),
            right: (
              <span className="dim">
                <Trans>
                  year {formatDay(usage.from)} — {formatDay(usage.to)}
                </Trans>
              </span>
            ),
          }}
        />
      }
    />
  );
}
