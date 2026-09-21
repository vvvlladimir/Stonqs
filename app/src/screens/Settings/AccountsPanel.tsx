import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { ArrowRightIcon, ChartPieSliceIcon, WalletIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { formatMoney } from "../../lib/format";
import { useNav } from "../../lib/nav";
import { affects, useAccounts, useInvalidate, usePortfolio } from "../../lib/queries";
import { Banner, Check, Form, List, ListRow, Num, Panel, Pending, QueryError } from "../../components/ui";
import type { AccountRow } from "../../lib/types";
import { inputOf } from "./model";

/**
 * Which accounts the reports are built from. The accounts themselves — opening,
 * renaming, closing — belong to the Accounts screen; this only draws the boundary.
 */
export function AccountsPanel() {
  const { t } = useLingui();
  const nav = useNav();
  const invalidate = useInvalidate();
  const portfolio = usePortfolio();
  const accounts = useAccounts();
  const [picked, setPicked] = useState<string[] | null>(null);

  const save = useMutation({
    mutationFn: api.portfolioSave,
    onSuccess: () => {
      setPicked(null);
      invalidate(...affects.portfolio);
    },
  });

  if (portfolio.isError) return <QueryError error={portfolio.error} />;
  if (accounts.isError) return <QueryError error={accounts.error} />;
  if (!portfolio.data || !accounts.data) return <Pending />;

  const ids = picked ?? portfolio.data.account_ids;
  const base = portfolio.data.base_currency;
  const toggle = (id: string) => setPicked(ids.includes(id) ? ids.filter((a) => a !== id) : [...ids, id]);

  const groups: Array<{ key: string; label: string; rows: AccountRow[] }> = [
    {
      key: "SECURITIES",
      label: t`Securities accounts`,
      rows: accounts.data.filter((a) => a.kind === "SECURITIES"),
    },
    {
      key: "DEPOSIT",
      label: t`Cash accounts`,
      rows: accounts.data.filter((a) => a.kind === "DEPOSIT"),
    },
  ];

  const line = (row: AccountRow) => (
    <ListRow
      key={row.id}
      pick={<Check checked={ids.includes(row.id)} onChange={() => toggle(row.id)} label={row.name} />}
      lead={row.kind === "SECURITIES" ? <ChartPieSliceIcon /> : <WalletIcon />}
      title={row.name}
      sub={[row.currency, row.is_active ? null : t`closed`].filter(Boolean).join(" · ")}
      value={row.value_base === null ? "—" : formatMoney(row.value_base, base)}
      meta={
        <Num dim>
          <Plural value={row.transaction_count} one="# transaction" other="# transactions" />
        </Num>
      }
    />
  );

  return (
    <Panel
      title={t`Accounts in the portfolio`}
      info={t`An account left outside the portfolio is kept but counted in no report.`}
      tools={
        <button className="btn btn--ghost btn--sm" onClick={() => nav.go("accounts")}>
          <Trans>Manage accounts</Trans> <ArrowRightIcon />
        </button>
      }
    >
      <Form
        onSubmit={() => save.mutate({ ...inputOf(portfolio.data), account_ids: ids })}
        busy={save.isPending}
        error={save.error}
        ready={picked !== null}
        onCancel={picked ? () => setPicked(null) : undefined}
      >
        {ids.length === 0 && (
          <Banner>
            <Trans>No account is in the portfolio, so every report comes out empty.</Trans>
          </Banner>
        )}
        {groups.map(
          (group) =>
            group.rows.length > 0 && (
              <div key={group.key}>
                <div className="group-label">{group.label}</div>
                <List>{group.rows.map(line)}</List>
              </div>
            ),
        )}
      </Form>
    </Panel>
  );
}
