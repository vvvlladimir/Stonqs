import { plural } from "@lingui/core/macro";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import {
  useIncomeOver,
  useIncomeTaxonomy,
  usePayments,
  usePositions,
  useTaxonomies,
} from "../../lib/queries";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { Page } from "../../components/Page";
import {
  Async,
  Choice,
  Empty,
  Metric,
  Metrics,
  Money,
  Panel,
  Pending,
  Percent,
  QueryError,
} from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { formatMoney, formatPercent, signOf } from "../../lib/format";
import type { PaymentPeriod, TransactionKind } from "../../lib/types";
import { ByTaxonomy } from "./ByTaxonomy";
import { CalendarPanel } from "./CalendarPanel";
import { Composition, KindLegend } from "./Composition";
import { Events } from "./Events";
import { MonthPanel } from "./MonthPanel";
import { Payers } from "./Payers";
import { PaymentsPanel } from "./PaymentsPanel";
import { kindOptions, kindSlots, pickMonth, ratio, valueBySecurity } from "./model";
import { useAsOf } from "../../lib/asOf";

/** Core provides dated income records and comparison totals; the UI only groups them. */
export function Income() {
  const { t, i18n } = useLingui();
  const asOf = useAsOf().date;
  const [period, setPeriod] = useState<PeriodId>("ONE_YEAR");
  const [kindFilter, setKindFilter] = useState("all");
  const [picked, setPicked] = useState<{ year: number; month: number } | null>(null);
  const [gridPeriod, setGridPeriod] = useState<PaymentPeriod>("QUARTER");
  const [treeId, setTreeId] = useState<string | null>(null);

  const kind = kindFilter === "all" ? null : (kindFilter as TransactionKind);
  const ranges = usePeriodRanges(asOf);
  const range = pickRange(ranges.data, period);
  // The calendar always shows the whole history; the period only drives the metrics.
  const whole = pickRange(ranges.data, "SINCE_INCEPTION");
  const income = useIncomeOver(range, kind);
  const history = useIncomeOver(whole, kind);
  const positions = usePositions(asOf);
  const payments = usePayments(range, gridPeriod);
  const taxonomies = useTaxonomies();
  const tree = treeId ?? taxonomies.data?.[0]?.id ?? null;
  const byTaxonomy = useIncomeTaxonomy(tree, range, kind);

  if (ranges.isError) return <QueryError error={ranges.error} />;
  if (ranges.isPending) return <Pending />;
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions — there is no income to compute.</Trans>
      </p>
    );

  const data = income.data;
  const all = history.data;
  const currency = data?.base_currency ?? "";
  const months = all?.by_month ?? [];
  const month = pickMonth(months, picked);
  const marketValue = positions.data?.total_value_base;

  return (
    <Page
      archetype="analysis"
      title={t`Income`}
      controls={
        <>
          <Choice
            label={t`Income kind`}
            value={kindFilter}
            onChange={setKindFilter}
            options={kindOptions(i18n)}
          />
          <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />
        </>
      }
      banner={income.isError ? <QueryError error={income.error} /> : undefined}
      metrics={
        <Metrics>
          <Metric
            label={t`Received in the period`}
            value={data ? <Money value={data.total.net_base} currency={currency} /> : "…"}
            hint={
              data
                ? t`${formatMoney(data.change_base, currency, { signed: true, compact: true })} versus the previous window`
                : undefined
            }
            tone={data ? signOf(data.change_base) : "neutral"}
            tip={t`What reached the cash accounts, net of withholding tax.`}
          />
          <Metric
            label={t`Tax withheld`}
            value={data ? <Money value={data.total.taxes_base} currency={currency} /> : "…"}
            hint={
              data
                ? t`${formatPercent(ratio(data.total.taxes_base, data.total.gross_base))} of the accrued amount`
                : undefined
            }
            tip={t`Withheld at the source, so it is not in what was received.`}
          />
          <Metric
            label={t`Accrued`}
            value={data ? <Money value={data.total.gross_base} currency={currency} /> : "…"}
            hint={data ? plural(data.total.events, { one: "# payment", other: "# payments" }) : undefined}
          />
          <Metric
            label={t`Portfolio yield`}
            value={
              data && marketValue ? (
                <Percent value={ratio(data.total.gross_base, marketValue)} digits={1} />
              ) : (
                "…"
              )
            }
            hint={
              marketValue
                ? t`against a value of ${formatMoney(marketValue, currency, { compact: true })}`
                : undefined
            }
            tip={t`Income in the window against current market value.`}
          />
        </Metrics>
      }
    >
      {data && all && data.total.events === 0 && all.total.events === 0 ? (
        <Empty title={t`No payments in this period`}>
          <Trans>
            Dividends, coupons and account interest appear here from transactions. If payments did happen,
            check the window or load a broker export under "Import".
          </Trans>
        </Empty>
      ) : (
        <>
          <CalendarPanel
            history={history}
            months={months}
            month={month}
            currency={currency}
            kind={kind}
            onPick={setPicked}
          />

          {month && all && (
            <MonthPanel
              month={month}
              months={months}
              events={all.events}
              currency={currency}
              onStep={setPicked}
            />
          )}

          <div className="grid-2 stack">
            <Panel
              title={t`What it is made of`}
              tools={all ? <KindLegend kinds={kindSlots(all)} /> : undefined}
            >
              <Async query={income}>
                {(data) => (
                  <Async query={history}>
                    {(all) => <Composition data={data} all={all} currency={currency} />}
                  </Async>
                )}
              </Async>
            </Panel>

            <Panel
              title={t`Who pays`}
              tools={
                data ? (
                  <span className="panel__note">
                    <Plural value={data.by_security.length} one="# payer" other="# payers" />
                  </span>
                ) : undefined
              }
            >
              <Async query={income}>
                {(data) => <Payers data={data} value={valueBySecurity(positions.data?.rows)} />}
              </Async>
            </Panel>
          </div>

          {tree && (
            <Panel
              title={t`Where it comes from`}
              info={t`Each payment counts under the category of whoever paid it, split by the same weights the tree uses for value.`}
              tools={
                taxonomies.data && taxonomies.data.length > 1 ? (
                  <Choice
                    label={t`Classification`}
                    value={tree}
                    onChange={setTreeId}
                    options={taxonomies.data.map((item) => ({ value: item.id, label: item.name }))}
                  />
                ) : undefined
              }
            >
              <Async query={byTaxonomy}>
                {(data) => (
                  <ByTaxonomy
                    data={data}
                    taxonomy={taxonomies.data?.find((item) => item.id === tree)}
                    currency={currency}
                  />
                )}
              </Async>
            </Panel>
          )}

          <PaymentsPanel query={payments} period={gridPeriod} onPeriod={setGridPeriod} />

          <Panel title={t`Payments`} info={t`Every income payment in the period, newest first.`}>
            <Async query={income}>{(data) => <Events events={data.events} currency={currency} />}</Async>
          </Panel>
        </>
      )}
    </Page>
  );
}
