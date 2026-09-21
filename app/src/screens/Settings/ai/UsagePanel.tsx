import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { useAiUsage } from "../../../lib/queries";
import { DataTable, Empty, ListRow, Num, Panel, Pending, QueryError } from "../../../components/ui";
import { formatDay, formatDecimal } from "../../../lib/format";
import type { AiUsageTotal } from "../../../lib/types";

/**
 * What has been asked of every model so far, counted by the provider. Tokens and not money on
 * purpose: prices change between releases, so a price list in the app would quietly lie — the
 * figures here are what the provider's own bill is computed from.
 *
 * Per model rather than per chat, and a deleted chat's requests still count: the question is
 * what the key was billed for (ADR-0041).
 */
export function UsagePanel() {
  const { t } = useLingui();
  const usage = useAiUsage();

  if (usage.isError) return <QueryError error={usage.error} />;
  if (!usage.data) return <Pending />;

  const count = (value: number) => formatDecimal(String(value), { digits: 0 });

  return (
    <Panel
      title={t`Tokens used`}
      info={t`Tokens as the provider counted them; for the price, see the provider's bill.`}
      table
    >
      <DataTable
        rows={usage.data}
        rowKey={(row) => `${row.provider}/${row.model}`}
        empty={
          <Empty title={t`Nothing asked yet`}>
            <Trans>Every answer, and every dashboard summary, is counted here once it arrives.</Trans>
          </Empty>
        }
        card={(row: AiUsageTotal) => (
          <ListRow
            key={`${row.provider}/${row.model}`}
            box
            title={row.model}
            sub={`${formatDay(day(row.first_at))} – ${formatDay(day(row.last_at))}`}
            value={count(row.usage.input_tokens + row.usage.output_tokens)}
            meta={<Plural value={row.requests} one="# request" other="# requests" />}
          />
        )}
        columns={[
          {
            key: "model",
            sort: (row) => row.model,
            header: t`Model`,
            align: "left",
            width: "14rem",
            className: "nm",
            cell: (row) => row.model,
          },
          {
            key: "requests",
            sort: (row) => row.requests,
            header: t`Requests`,
            ariaLabel: t`Requests, not answers: one answer that reads your data costs several.`,
            cell: (row) => <Num>{count(row.requests)}</Num>,
          },
          {
            key: "input",
            sort: (row) => row.usage.input_tokens,
            header: t`Sent`,
            cell: (row) => <Num>{count(row.usage.input_tokens)}</Num>,
          },
          {
            key: "cached",
            sort: (row) => row.usage.cached_tokens,
            header: t`From cache`,
            ariaLabel: t`Part of what was sent that the provider served from its own cache, usually cheaper.`,
            only: "wide",
            cell: (row) => <Num dim>{count(row.usage.cached_tokens)}</Num>,
          },
          {
            key: "output",
            sort: (row) => row.usage.output_tokens,
            header: t`Received`,
            cell: (row) => <Num>{count(row.usage.output_tokens)}</Num>,
          },
          {
            key: "reasoning",
            sort: (row) => row.usage.reasoning_tokens,
            header: t`Thinking`,
            ariaLabel: t`Part of what was received that the model thought but never showed.`,
            only: "wide",
            cell: (row) => <Num dim>{count(row.usage.reasoning_tokens)}</Num>,
          },
          {
            key: "last",
            sort: (row) => row.last_at,
            header: t`Last used`,
            align: "left",
            only: "wide",
            width: "9rem",
            cell: (row) => formatDay(day(row.last_at)),
          },
        ]}
      />
    </Panel>
  );
}

/** `first_at`/`last_at` are timestamps; the table shows the day, which is the part that answers
 * "when was this last used". */
function day(at: string): string {
  return at.slice(0, 10);
}
