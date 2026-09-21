import { useMutation } from "@tanstack/react-query";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { keys, useCoverage, useInvalidate, useSettings } from "../../lib/queries";
import { MarketRefresh } from "../../components/domain/MarketRefresh";
import { Badge, CheckField, DataTable, Empty, Field, Form, ListRow, Num, Panel } from "../../components/ui";
import type { DataCoverage } from "../../lib/types";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";
import { SourcesPanel } from "./market/SourcesPanel";

/** A quote older than this is called out: a week covers any market holiday. */
const STALE_DAYS = 7;

/** Where quotes come from, when they are fetched, and how far each instrument is covered. */
export function MarketDataPanel() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const coverage = useCoverage();
  const settings = useSettings();

  const save = useMutation({
    mutationFn: api.settingsSave,
    onSuccess: () => invalidate(keys.settings()),
  });

  const current = settings.data;
  const rows = coverage.data ?? [];
  const stale = rows.filter((row) => (row.stale_days ?? 0) >= STALE_DAYS).length;

  return (
    <>
      <Panel title={t`Refresh and schedule`} info={t`When quotes and exchange rates are fetched.`}>
        <MarketRefresh />

        {current && (
          <Form>
            <CheckField
              label={t`Fetch data when the app starts`}
              checked={current.auto_refresh_on_start}
              onChange={(on) => save.mutate({ ...current, auto_refresh_on_start: on })}
            />

            <Field
              label={t`Not more often than once every, hours`}
              hint={t`A startup within this window skips the request and uses what is already stored.`}
            >
              <input
                inputMode="numeric"
                value={current.refresh_min_interval_hours}
                onChange={(e) =>
                  save.mutate({
                    ...current,
                    refresh_min_interval_hours: Number(e.target.value) || 0,
                  })
                }
              />
            </Field>
          </Form>
        )}
      </Panel>

      <SourcesPanel />

      <Panel
        table
        title={t`Coverage`}
        note={
          rows.length > 0 ? (
            stale > 0 ? (
              <Plural value={stale} one="# instrument out of date" other="# instruments out of date" />
            ) : (
              <Plural
                value={rows.length}
                one="# instrument, all up to date"
                other="# instruments, all up to date"
              />
            )
          ) : undefined
        }
      >
        <DataTable
          rows={rows}
          rowKey={(row) => row.security_id}
          empty={
            <Empty title={t`Nothing downloaded yet`}>
              <Trans>Refresh above, and every identified instrument gets its quote history.</Trans>
            </Empty>
          }
          card={(row) => (
            <ListRow
              key={row.security_id}
              box
              title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
              sub={[row.venue ?? row.mic, row.data_source ?? t`manual`].filter(Boolean).join(" · ")}
              value={row.last_quote ? formatDay(row.last_quote) : t`no quotes`}
              meta={staleness(row)}
            />
          )}
          columns={[
            {
              key: "symbol",
              sort: (row) => row.symbol,
              header: t`Instrument`,
              align: "left",
              className: "nm",
              cell: (row) => <SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>,
            },
            {
              key: "venue",
              sort: (row) => row.venue ?? row.mic,
              header: t`Venue`,
              align: "left",
              only: "wide",
              cell: (row) => row.venue ?? row.mic ?? "—",
            },
            {
              key: "source",
              sort: (row) => row.data_source,
              header: t`Provider`,
              align: "left",
              cell: (row) => row.data_source ?? t`manual`,
            },
            {
              key: "count",
              sort: (row) => row.quote_count,
              header: t`Quotes`,
              width: "90px",
              only: "wide",
              cell: (row) => <Num dim>{row.quote_count}</Num>,
            },
            {
              key: "first",
              sort: (row) => row.first_quote,
              header: t`From`,
              align: "left",
              only: "wide",
              cell: (row) => (row.first_quote ? formatDay(row.first_quote) : "—"),
            },
            {
              key: "last",
              sort: (row) => row.last_quote,
              header: t`Last quote`,
              align: "left",
              width: "170px",
              cell: (row) => (
                <span className="inline">
                  {row.last_quote ? formatDay(row.last_quote) : "—"}
                  {staleness(row)}
                </span>
              ),
            },
          ]}
        />

        <p className="muted">
          <Trans>
            A valuation on a date without a quote takes the last one known before it. So past the date in this
            table the portfolio value stops changing — not because the market stands still, but because the
            data ends.
          </Trans>
        </p>
      </Panel>
    </>
  );
}

/** Age of the newest quote, shown only once it is old enough to distort a valuation. */
function staleness(row: DataCoverage) {
  if (row.stale_days === null || row.stale_days < STALE_DAYS) return null;
  return (
    <Badge tone="warn">
      <Plural value={row.stale_days} one="# day old" other="# days old" />
    </Badge>
  );
}
