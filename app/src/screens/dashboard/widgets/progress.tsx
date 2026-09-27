import { msg, plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, Money, Percent, Progress } from "../../../components/ui";
import { useFire, useGoals, useLimits } from "../../../lib/queries";
import { formatDay, formatMoney } from "../../../lib/format";
import type { FireProjection } from "../../../lib/types";
import { fireCfg, trackOf, type WidgetProps } from "./model";

export function ProgressWidget(props: WidgetProps) {
  switch (trackOf(props.widget.cfg)) {
    case "limit":
      return <LimitWidget {...props} />;
    case "fire":
      return <FireWidget {...props} />;
    default:
      return <GoalWidget {...props} />;
  }
}

/**
 * How far the portfolio is from covering a year of spending, and when this pace would get
 * there. Every figure but today's value is an assumption — the return is not this portfolio's
 * measured return and is never read from it.
 */
function FireWidget({ widget }: WidgetProps) {
  const { t, i18n } = useLingui();
  const assumptions = fireCfg(widget.cfg);
  const query = useFire(assumptions);

  if (assumptions.annual_spending === "")
    return (
      <p className="muted">
        <Trans>Name the yearly spending this portfolio should cover.</Trans>
      </p>
    );

  return (
    <Async query={query}>
      {(data) => {
        const months = data.months_to_target;
        return (
          <Progress
            value={<Percent value={data.progress} digits={0} />}
            share={data.progress}
            legend={{
              left:
                months === null
                  ? t`not within a hundred years at this pace`
                  : months === 0
                    ? t`already there`
                    : i18n._(
                        msg`${plural(Math.round(months / 12), { one: "# year", other: "# years" })} to go`,
                      ),
            }}
            note={<FireNote data={data} />}
          />
        );
      }}
    </Async>
  );
}

function FireNote({ data }: { data: FireProjection }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  // Plain text: `Progress` puts it in its own note line, which already draws the rule above it.
  return (
    <>
      {t`${formatMoney(data.current_base, currency, { compact: true })} of ${formatMoney(data.target_base, currency, { compact: true })}`}
      {data.target_date && ` · ${formatDay(data.target_date)}`}
    </>
  );
}

/**
 * One goal as a track: where it is, and the single figure that answers "is this enough".
 * Not scoped — a goal carries the accounts it counts, so the tile's own source would mean
 * nothing here.
 */
function GoalWidget({ widget, date }: WidgetProps) {
  const { t } = useLingui();
  const query = useGoals(date);
  const wanted = typeof widget.cfg.goal === "string" ? widget.cfg.goal : "";

  return (
    <Async query={query}>
      {(rows) => {
        const row = rows.find((r) => r.goal.id === wanted) ?? rows[0];
        if (!row)
          return (
            <p className="muted">
              <Trans>No goals yet — one is added on the Plans screen.</Trans>
            </p>
          );
        const { progress } = row;
        return (
          <Progress
            value={<Percent value={progress.progress} digits={0} />}
            share={progress.progress}
            barTone={progress.on_track === false ? "neg" : undefined}
            legend={{
              left: (
                <>
                  <Money value={progress.current_base} currency={row.goal.currency} /> {t`of`}{" "}
                  <Money value={progress.target_base} currency={row.goal.currency} />
                </>
              ),
              right:
                progress.months_left !== null
                  ? plural(progress.months_left, { one: "# month left", other: "# months left" })
                  : progress.projected_date !== null
                    ? t`at this pace, ${formatDay(progress.projected_date)}`
                    : t`no date and no pace stated`,
            }}
            note={
              progress.required_monthly_base !== null && (
                <>
                  <Money value={progress.required_monthly_base} currency={row.goal.currency} />{" "}
                  {t`a month needed`}
                </>
              )
            }
          />
        );
      }}
    </Async>
  );
}

/** One contribution limit as a track: spent against allowed, over its own limit year. */
function LimitWidget({ widget, date }: WidgetProps) {
  const { t } = useLingui();
  const query = useLimits(date);
  const wanted = typeof widget.cfg.limit === "string" ? widget.cfg.limit : "";

  return (
    <Async query={query}>
      {(rows) => {
        const usage = rows.find((r) => r.limit_id === wanted) ?? rows[0];
        if (!usage)
          return (
            <p className="muted">
              <Trans>No limits set — one is added on the Accounts screen.</Trans>
            </p>
          );
        const over = Number(usage.share) > 1;
        return (
          <Progress
            value={<Percent value={usage.share} digits={0} />}
            share={usage.share}
            barTone={over ? "neg" : undefined}
            legend={{
              left: (
                <>
                  <Money value={usage.remaining} currency={usage.currency} /> {t`left of`}{" "}
                  <Money value={usage.allowance} currency={usage.currency} />
                </>
              ),
            }}
            note={t`of the allowance used`}
          />
        );
      }}
    </Async>
  );
}
