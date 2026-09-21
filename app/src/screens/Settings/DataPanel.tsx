import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  ArrowCounterClockwiseIcon,
  ArrowRightIcon,
  CopyIcon,
  FlaskIcon,
  TrashIcon,
} from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { useNav } from "../../lib/nav";
import {
  affects,
  keys,
  useAccounts,
  useCoverage,
  useImportTemplates,
  useInvalidate,
  useSecurities,
} from "../../lib/queries";
import {
  Badge,
  Buttons,
  Empty,
  ErrorText,
  List,
  ListRow,
  Metric,
  Metrics,
  Panel,
  useToast,
} from "../../components/ui";
import type { AppStatus } from "../../lib/types";

/** What the portfolio is made of, where it is stored, and the layouts that fill it. */
export function DataPanel({ status }: { status: AppStatus }) {
  const { t } = useLingui();
  const toast = useToast();
  const nav = useNav();
  const invalidate = useInvalidate();
  const queryClient = useQueryClient();
  const accounts = useAccounts();
  const securities = useSecurities();
  const coverage = useCoverage();
  const templates = useImportTemplates();

  const seed = useMutation({
    mutationFn: api.devSeedDemo,
    onSuccess: () => invalidate(...affects.transactions, ...affects.accounts),
  });

  const forget = useMutation({
    mutationFn: api.importTemplateDelete,
    onSuccess: (data) => queryClient.setQueryData(keys.importTemplates(), data),
  });

  const restore = useMutation({
    mutationFn: api.importPresetsRestore,
    onSuccess: (data) => queryClient.setQueryData(keys.importTemplates(), data),
  });

  const quotes = (coverage.data ?? []).reduce((sum, row) => sum + row.quote_count, 0);

  const copyPath = () => {
    void navigator.clipboard.writeText(status.db_path);
    toast(t`Path copied`);
  };

  return (
    <>
      <Panel title={t`What is stored`} info={t`What the reports are computed from.`}>
        <Metrics pair inset>
          <Metric label={t`Accounts`} value={accounts.data?.length ?? "—"} />
          <Metric label={t`Instruments`} value={securities.data?.length ?? "—"} />
          <Metric label={t`Quotes`} value={quotes} />
          <Metric
            label={t`First transaction`}
            value={status.inception ? formatDay(status.inception) : t`none yet`}
          />
        </Metrics>

        <Buttons>
          <button className="btn btn--ghost btn--sm" onClick={() => nav.go("import")}>
            <Trans>Import a file</Trans> <ArrowRightIcon />
          </button>
          <button className="btn btn--ghost btn--sm" onClick={() => nav.go("transactions")}>
            <Trans>Transactions</Trans> <ArrowRightIcon />
          </button>
          <button className="btn btn--ghost btn--sm" onClick={() => nav.go("securities")}>
            <Trans>Instruments</Trans> <ArrowRightIcon />
          </button>
        </Buttons>
      </Panel>

      <Panel
        title={t`Import layouts`}
        info={t`A saved layout reads the next export from the same broker without asking again.`}
        tools={
          <button
            className="btn btn--ghost btn--sm"
            disabled={restore.isPending}
            title={t`Restore the removed shipped presets`}
            onClick={() => restore.mutate()}
          >
            <ArrowCounterClockwiseIcon /> <Trans>Restore shipped</Trans>
          </button>
        }
      >
        {templates.data && templates.data.length === 0 ? (
          <Empty title={t`No layouts`}>
            <Trans>The import wizard saves one at the end, and every later file reuses it.</Trans>
          </Empty>
        ) : (
          <List>
            {(templates.data ?? []).map((template) => (
              <ListRow
                key={template.name}
                title={template.name}
                end={
                  template.source === "BUILTIN" ? (
                    <Badge tone="info">
                      <Trans>Shipped</Trans>
                    </Badge>
                  ) : undefined
                }
                actions={
                  <button
                    className="iconbtn iconbtn--sm iconbtn--danger"
                    title={
                      template.source === "BUILTIN"
                        ? t`Remove the shipped preset from the list`
                        : t`Delete this layout`
                    }
                    disabled={forget.isPending}
                    onClick={() => forget.mutate(template.name)}
                  >
                    <TrashIcon />
                  </button>
                }
              />
            ))}
          </List>
        )}
        <ErrorText error={forget.error ?? restore.error} />
      </Panel>

      <Panel title={t`Database`} info={t`The whole portfolio is one file; a copy of it is a full backup.`}>
        <List>
          <ListRow
            wrap
            title={<span className="mono">{status.db_path}</span>}
            end={
              <button className="btn btn--ghost btn--sm" onClick={copyPath}>
                <CopyIcon /> <Trans>Copy path</Trans>
              </button>
            }
          />
        </List>
      </Panel>

      {status.dev_build && (
        <Panel title={t`Developer`} info={t`Shown only in a development build.`}>
          <Buttons>
            <button
              className="btn btn--ghost btn--sm"
              disabled={seed.isPending}
              onClick={() => seed.mutate()}
            >
              <FlaskIcon /> <Trans>Add demo data</Trans>
            </button>
          </Buttons>
          <ErrorText error={seed.error} />
        </Panel>
      )}
    </>
  );
}
