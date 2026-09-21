import { useLingui } from "@lingui/react/macro";
import { Instrument } from "../../components/domain/Instrument";
import {
  DataTable,
  ListRow,
  Money,
  Percent,
  useMenu,
  type Column as TableColumn,
  type MenuItem,
  type SortState,
} from "../../components/ui";
import type { WatchRow } from "../../lib/types";
import type { WatchColumn, WatchCtx } from "./columns";

export function WatchTable({
  rows,
  columns,
  currency,
  ctx,
  sort,
  onSortChange,
  menu,
  itemsFor,
}: {
  rows: WatchRow[];
  columns: WatchColumn[];
  currency: string;
  ctx: (row: WatchRow) => WatchCtx;
  /** Ordering outlives the screen, so the caller stores it. */
  sort: SortState | null;
  onSortChange: (sort: SortState | null) => void;
  menu: ReturnType<typeof useMenu>;
  itemsFor: (row: WatchRow) => MenuItem[];
}) {
  const { t, i18n } = useLingui();
  const instrument = (row: WatchRow) => (
    <Instrument id={row.security_id} symbol={row.symbol} name={row.name} kind={row.kind} />
  );

  return (
    <DataTable
      rows={rows}
      sort={sort}
      onSortChange={onSortChange}
      rowKey={(row) => row.security_id}
      rowProps={(row) => menu.row(row.security_id, itemsFor(row))}
      columns={[
        {
          key: "instrument",
          header: t`Instrument`,
          align: "left",
          sort: (row) => row.name || row.symbol,
          cell: instrument,
        },
        ...columns.map((column): TableColumn<WatchRow> => ({
          key: column.id,
          header: column.label(i18n, currency),
          ariaLabel: column.tip?.(i18n),
          align: column.align,
          width: column.width,
          cellClass: (row) => column.tone?.(row, ctx(row)) ?? "",
          sort: column.sort && ((row: WatchRow) => column.sort!(row, ctx(row))),
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
          value={row.price && row.currency ? <Money value={row.price} currency={row.currency} /> : "—"}
          meta={row.day_change ? <Percent value={row.day_change} signed /> : undefined}
          actions={menu.button(row.security_id, itemsFor(row))}
          foot={
            <>
              {row.period_return ? (
                <Percent value={row.period_return} signed />
              ) : (
                <span className="dim">—</span>
              )}
              {row.nearest_level && (
                <span>
                  <Money value={row.nearest_level.level} currency={row.nearest_level.currency} /> ·{" "}
                  <Percent value={row.nearest_level.distance} signed tone={false} />
                </span>
              )}
            </>
          }
        />
      )}
    />
  );
}
