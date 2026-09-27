import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { DataTable, Empty, Num, Rate } from "../../components/ui";
import { formatMonth } from "../../lib/format";
import type { Drawdown } from "../../lib/types";
import { dayDelta, days } from "./days";

/** The deepest falls first, each with how long it took to fall and to recover. */
export function DrawdownTable({ episodes }: { episodes: Drawdown[] }) {
  const { t, i18n } = useLingui();
  return (
    <DataTable
      variant="rows"
      rows={byDepth(episodes)}
      rowKey={(episode) => `${episode.peak}-${episode.trough}`}
      empty={
        <Empty title={t`No drawdowns`}>
          <Trans>Over the selected period the value never fell below a previous peak.</Trans>
        </Empty>
      }
      columns={[
        {
          key: "span",
          sort: (episode) => episode.peak,
          header: t`Period`,
          align: "left",
          cell: (episode) => span(i18n, episode),
          card: "title",
        },
        {
          key: "depth",
          sort: (episode) => episode.depth,
          header: t`Depth`,
          className: "neg",
          card: "value",
          cell: (episode) => <Rate value={episode.depth} digits={2} />,
        },
        {
          key: "fall",
          sort: (episode) => dayDelta(episode.peak, episode.trough),
          header: (
            <span data-tip={t`Days from peak to trough.`}>
              <Trans>Fall</Trans>
            </span>
          ),
          factLabel: t`fall`,
          cell: (episode) => <Num>{days(episode.peak, episode.trough)}</Num>,
        },
        {
          key: "recovery",
          sort: (episode) => (episode.recovered ? dayDelta(episode.trough, episode.recovered) : null),
          header: (
            <span data-tip={t`Days from the trough back to the peak.`}>
              <Trans>Recovery</Trans>
            </span>
          ),
          factLabel: t`recovery`,
          cell: (episode) =>
            episode.recovered ? <Num>{days(episode.trough, episode.recovered)}</Num> : t`not recovered`,
        },
        {
          key: "total",
          sort: (episode) => (episode.recovered ? dayDelta(episode.peak, episode.recovered) : null),
          header: t`Total`,
          card: "none",
          cell: (episode) => (episode.recovered ? <Num>{days(episode.peak, episode.recovered)}</Num> : "—"),
        },
      ]}
    />
  );
}

/** Sort deeper drawdowns first; `depth` is negative. */
function byDepth(episodes: Drawdown[]): Drawdown[] {
  return [...episodes].sort((a, b) => a.depth - b.depth);
}

/** An open drawdown is reported explicitly rather than as missing data. */
function span(i18n: I18n, episode: Drawdown): string {
  const [year, month] = episode.peak.split("-").map(Number);
  const from = formatMonth(year, month);
  if (!episode.recovered) return i18n._(msg`${from} — now`);
  const [ry, rm] = episode.recovered.split("-").map(Number);
  return `${from} — ${formatMonth(ry, rm)}`;
}
