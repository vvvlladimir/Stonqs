import { Trans, useLingui } from "@lingui/react/macro";
import { Instrument } from "../../components/domain/Instrument";
import {
  DataTable,
  Empty,
  ListRow,
  Money,
  Percent,
  Quantity,
  useMenu,
  type Column as TableColumn,
  type MenuItem,
  type SortState,
} from "../../components/ui";
import type {
  CostBasisMethod,
  PositionCostRow,
  PositionReturnRow,
  PositionRow,
  SecurityRow,
} from "../../lib/types";
import type { Column } from "../../components/domain/positionColumns";

export function PositionsTable({
  rows,
  columns,
  currency,
  returnOf,
  costOf,
  securityOf,
  method,
  sort,
  onSortChange,
  menu,
  itemsFor,
}: {
  rows: PositionRow[];
  columns: Column[];
  currency: string;
  /** Period returns keyed by security; empty while that query is loading. */
  returnOf: Map<string, PositionReturnRow>;
  /** Both cost-basis methods keyed by security; empty while no such column is shown. */
  costOf: Map<string, PositionCostRow>;
  securityOf: Map<string, SecurityRow>;
  /** The method the portfolio is kept under; the row already holds that one's figures. */
  method?: CostBasisMethod;
  /** Ordering outlives the screen, so the caller stores it. */
  sort: SortState | null;
  onSortChange: (sort: SortState | null) => void;
  menu: ReturnType<typeof useMenu>;
  itemsFor: (row: PositionRow) => MenuItem[];
}) {
  const { t, i18n } = useLingui();
  const ctx = (row: PositionRow) => ({
    currency,
    i18n,
    period: returnOf.get(row.security_id),
    cost: costOf.get(row.security_id),
    security: securityOf.get(row.security_id),
    own: method,
  });
  const instrument = (row: PositionRow) => (
    <Instrument
      id={row.security_id}
      symbol={row.symbol}
      name={row.name}
      kind={securityOf.get(row.security_id)?.kind}
    />
  );

  // Each figure states the width it needs and the name lives on what is left; once the user
  // switches on more columns than fit, `DataTable` scrolls them sideways.
  return (
    <DataTable
      rows={rows}
      sort={sort}
      onSortChange={onSortChange}
      rowKey={(row) => row.security_id}
      rowProps={(row) => menu.row(row.security_id, itemsFor(row))}
      empty={
        <Empty title={t`Nothing found`}>
          <Trans>No position matches the filter.</Trans>
        </Empty>
      }
      columns={[
        {
          key: "instrument",
          header: t`Instrument`,
          align: "left",
          sort: (row) => row.name || row.symbol,
          cell: instrument,
        },
        ...columns.map((column): TableColumn<PositionRow> => ({
          key: column.id,
          header: column.label(i18n, currency),
          ariaLabel: column.tip?.(i18n),
          align: column.align,
          width: column.width,
          cellClass: (row) => column.tone?.(row, ctx(row)) ?? "",
          sort: column.sort && ((row: PositionRow) => column.sort!(row, ctx(row))),
          cell: (row) => column.cell(row, ctx(row)),
        })),
        {
          key: "acts",
          align: "left",
          className: "acts",
          width: "52px",
          cell: (row) => menu.button(row.security_id, itemsFor(row)),
        },
      ]}
      card={(row) => (
        <ListRow
          as="li"
          box
          {...menu.row(row.security_id, itemsFor(row))}
          top
          title={instrument(row)}
          value={<Money value={row.market_value_base} currency={currency} />}
          meta={<Percent value={row.weight} digits={1} />}
          actions={menu.button(row.security_id, itemsFor(row))}
          foot={
            <>
              <span>
                <Quantity value={row.quantity} /> × <Money value={row.price} currency={row.currency} />
              </span>
              <Money value={row.unrealized_pnl_base} signed />
            </>
          }
        />
      )}
    />
  );
}
