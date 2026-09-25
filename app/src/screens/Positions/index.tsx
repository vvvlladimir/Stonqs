import { scopeLabel } from "../../components/domain/scopeLabel";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { ColumnsIcon } from "@phosphor-icons/react";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import {
  usePortfolio,
  usePositionReturns,
  usePositions,
  usePositionsCostBasis,
  useScope,
  useSecurities,
} from "../../lib/queries";
import { DEFAULT_UI, useUiState } from "../../lib/uiState";
import { SECURITY_KIND_FILTERS } from "../../lib/kinds";
import { formatMoney } from "../../lib/format";
import { Page } from "../../components/Page";
import { useSecurityCard } from "../../components/domain/SecurityCardProvider";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { useNav } from "../../lib/nav";
import {
  Choice,
  Empty,
  Panel,
  Pending,
  QueryError,
  SearchBox,
  ColumnPicker,
  useMenu,
  type MenuItem,
} from "../../components/ui";
import type { PositionRow, SecurityKind, SecurityRow } from "../../lib/types";
import { PositionsTable } from "./PositionsTable";
import {
  GROUPS,
  needsCostQuery,
  orderColumns,
  resolveColumnIds,
  useAllColumns,
} from "../../components/domain/positionColumns";
import { useAsOf } from "../../lib/asOf";

export function Positions() {
  const { t, i18n } = useLingui();
  const date = useAsOf().date;
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<string>("ALL");
  const [picking, setPicking] = useState(false);
  // Value is read off `date`; a return has to be measured over something, hence a period too.
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");
  const menu = useMenu();
  const card = useSecurityCard();
  const nav = useNav();

  const positions = usePositions(date);
  const securities = useSecurities();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, period);
  // The same query the performance screen uses, so one history pass serves both screens.
  const returns = usePositionReturns(range);
  const scope = useScope();
  // The scope's wording lives in one place; the host only names the subject.
  const current = scope.data?.options.find(
    (o) => o.kind === scope.data?.scope.kind && o.id === scope.data?.scope.id,
  );
  const { ui, save } = useUiState();
  const allColumns = useAllColumns();
  const portfolio = usePortfolio();
  const method = portfolio.data?.cost_basis_method;
  // The portfolio's own method is already in the position row, so only a column asking for
  // the other one — or a unit price — is worth the host's second holdings pass.
  const chosen = resolveColumnIds(ui.position_columns, method);
  const costs = usePositionsCostBasis(needsCostQuery(chosen, method) ? date : null);

  if (positions.isError) return <QueryError error={positions.error} />;
  if (positions.isPending) return <Pending />;

  const { rows, base_currency, total_value_base } = positions.data;
  const securityOf = new Map<string, SecurityRow>((securities.data ?? []).map((s) => [s.id, s]));

  // Offer filters only for security types present in the portfolio.
  const present = [...new Set(rows.map((row) => securityOf.get(row.security_id)?.kind).filter(Boolean))];

  const needle = query.trim().toLowerCase();
  const shown = rows.filter((row) => {
    const security = securityOf.get(row.security_id);
    if (kind !== "ALL" && security?.kind !== kind) return false;
    if (!needle) return true;
    return (
      row.symbol.toLowerCase().includes(needle) ||
      row.name.toLowerCase().includes(needle) ||
      (security?.isin ?? "").toLowerCase().includes(needle)
    );
  });

  const columns = orderColumns(allColumns, chosen);
  const returnOf = new Map((returns.data ?? []).map((r) => [r.security_id, r]));
  const costOf = new Map((costs.data?.rows ?? []).map((r) => [r.security_id, r]));

  const itemsFor = (row: PositionRow): MenuItem[] => {
    const isin = securityOf.get(row.security_id)?.isin ?? null;
    return [
      { label: t`Instrument card…`, onSelect: () => card.open(row.security_id) },
      {
        label: t`Transactions for this instrument`,
        onSelect: () => nav.go("transactions", row.symbol),
      },
      {
        label: t`Copy ticker`,
        onSelect: () => navigator.clipboard.writeText(row.symbol),
      },
      {
        label: t`Copy ISIN`,
        onSelect: () => isin && navigator.clipboard.writeText(isin),
        disabled: isin === null,
        title: isin === null ? t`This instrument has no ISIN` : undefined,
      },
    ];
  };

  return (
    <Page
      archetype="registry"
      title={t`Positions`}
      summary={`${plural(rows.length, { one: "# instrument", other: "# instruments" })} · ${
        current ? scopeLabel(i18n, current) : t`whole portfolio`
      } · ${formatMoney(total_value_base, base_currency)}`}
      controls={<PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />}
      actions={
        <button type="button" className="iconbtn" onClick={() => setPicking(true)}>
          <ColumnsIcon /> <Trans>Columns</Trans>
        </button>
      }
      filters={
        <>
          <SearchBox value={query} onChange={setQuery} placeholder={t`Ticker, name or ISIN`} />
          {present.length > 1 && (
            <Choice
              fixed
              label={t`Instrument kind`}
              value={kind}
              onChange={setKind}
              options={[
                { value: "ALL", label: t`All` },
                ...present.map((k) => ({
                  value: k as string,
                  label: i18n._(SECURITY_KIND_FILTERS[k as SecurityKind]),
                })),
              ]}
            />
          )}
        </>
      }
    >
      {rows.length === 0 ? (
        <Empty title={t`No open positions`}>
          <Trans>
            A position comes from transactions: add a purchase on the "Transactions" screen, or load a broker
            export under "Import".
          </Trans>
        </Empty>
      ) : (
        <Panel table>
          <PositionsTable
            rows={shown}
            columns={columns}
            currency={base_currency}
            returnOf={returnOf}
            costOf={costOf}
            securityOf={securityOf}
            method={method}
            sort={ui.position_sort}
            onSortChange={(position_sort) => save((ui) => ({ ...ui, position_sort }))}
            menu={menu}
            itemsFor={itemsFor}
          />
        </Panel>
      )}

      {picking && (
        <ColumnPicker
          columns={allColumns.map((c) => ({
            id: c.id,
            label: c.label(i18n, base_currency),
            group: c.group,
            tip: c.tip?.(i18n),
          }))}
          groups={GROUPS.map((g) => ({ id: g.id, label: i18n._(g.label) }))}
          selected={chosen}
          onChange={(position_columns) => save((ui) => ({ ...ui, position_columns }))}
          onReset={() => save((ui) => ({ ...ui, position_columns: DEFAULT_UI.position_columns }))}
          onClose={() => setPicking(false)}
        />
      )}

      {menu.node}
    </Page>
  );
}
