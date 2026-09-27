import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { Badge, ListRow, Money, Progress } from "../../components/ui";
import { formatDay, formatPercent } from "../../lib/format";
import type { GoalRow } from "../../lib/types";

/** One goal: how far along it is, and the one figure that answers "is this enough". */
export function GoalCard({
  row,
  onEdit,
  onDelete,
}: {
  row: GoalRow;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const { goal, progress } = row;
  const currency = goal.currency;

  const pace =
    progress.required_monthly_base !== null ? (
      <Trans>
        <Money value={progress.required_monthly_base} currency={currency} /> a month to arrive on time
      </Trans>
    ) : progress.projected_date !== null ? (
      <Trans>at this pace, {formatDay(progress.projected_date)}</Trans>
    ) : progress.monthly_base !== null ? (
      <Trans>this pace does not arrive</Trans>
    ) : (
      <Trans>no monthly amount stated</Trans>
    );

  return (
    <ListRow
      box
      top
      title={goal.name}
      sub={row.account_names.length > 0 ? row.account_names.join(" · ") : <Trans>the whole portfolio</Trans>}
      value={<Money value={progress.current_base} currency={currency} />}
      meta={
        <Trans>
          of <Money value={progress.target_base} currency={currency} />
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
        // The same block the dashboard's Progress tile draws: above the target the track is full
        // and the overshoot is a figure, never a longer bar.
        <Progress
          size="sm"
          share={progress.progress}
          barTone={progress.on_track === false ? "neg" : undefined}
          legend={{
            left: (
              <>
                {formatPercent(progress.progress)}
                {progress.months_left !== null && (
                  <>
                    {" · "}
                    {plural(progress.months_left, { one: "# month left", other: "# months left" })}
                  </>
                )}
                {progress.on_track !== null && (
                  <>
                    {" · "}
                    <Badge tone={progress.on_track ? "in" : "warn"}>
                      {progress.on_track ? t`on track` : t`behind`}
                    </Badge>
                  </>
                )}
              </>
            ),
            right: <span className="dim">{pace}</span>,
          }}
        />
      }
    />
  );
}
