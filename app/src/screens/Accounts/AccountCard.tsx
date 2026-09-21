import { ChartPieSliceIcon, WalletIcon } from "@phosphor-icons/react";
import { Plural, useLingui } from "@lingui/react/macro";
import { ListRow, Num, type MenuItem, type useMenu } from "../../components/ui";
import { Logo } from "../../components/domain/Instrument";
import { formatMoney } from "../../lib/format";
import { accountKindLabel } from "../../lib/kinds";
import type { AccountRow } from "../../lib/types";

export function AccountCard({
  row,
  base,
  menu,
  onEdit,
  onDelete,
}: {
  row: AccountRow;
  base: string;
  menu: ReturnType<typeof useMenu>;
  onEdit: (row: AccountRow) => void;
  onDelete: (row: AccountRow) => void;
}) {
  const { t, i18n } = useLingui();
  const depot = row.kind === "SECURITIES";
  // A card is a narrow grid cell, so the subtitle is kept short enough to stay on one line.
  const meta = [
    accountKindLabel(i18n, row.kind),
    row.currency,
    depot && row.reference_name ? t`through ${row.reference_name}` : null,
    row.is_active ? null : t`closed`,
    row.in_portfolio ? null : t`outside the portfolio`,
  ].filter(Boolean);

  const items: MenuItem[] = [
    { label: t`Edit…`, onSelect: () => onEdit(row) },
    { label: t`Delete`, danger: true, onSelect: () => onDelete(row) },
  ];

  return (
    <ListRow
      box
      big
      top
      {...menu.row(row.id, items)}
      lead={<Logo symbol="" name={row.name} icon={depot ? <ChartPieSliceIcon /> : <WalletIcon />} />}
      title={row.name}
      sub={meta.join(" · ")}
      actions={menu.button(row.id, items)}
      foot={
        <>
          <span className="row__val num">{valueOf(row, base)}</span>
          <span className="spacer" />
          <Num dim>
            <Plural value={row.transaction_count} one="# transaction" other="# transactions" />
          </Num>
        </>
      }
    />
  );
}

/** Core supplies the value; a dash means quote or FX data is missing. */
function valueOf(row: AccountRow, base: string): string {
  return row.value_base === null ? "—" : formatMoney(row.value_base, base);
}
