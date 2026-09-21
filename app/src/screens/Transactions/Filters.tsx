import { useLingui } from "@lingui/react/macro";
import { Choice, SearchBox } from "../../components/ui";
import { EDITABLE_TRANSACTION_KINDS, transactionLabel } from "../../lib/kinds";
import type { TransactionFilter, TransactionKind } from "../../lib/types";

/** Search over loaded rows plus the two filters the core applies itself. */
export function Filters({
  query,
  onQuery,
  filter,
  onFilter,
  years,
}: {
  query: string;
  onQuery: (query: string) => void;
  filter: TransactionFilter;
  onFilter: (filter: TransactionFilter) => void;
  years: string[];
}) {
  const { t, i18n } = useLingui();
  return (
    <>
      <SearchBox value={query} onChange={onQuery} placeholder={t`Instrument, kind or note`} />

      <Choice
        label={t`Transaction kind`}
        placeholder={t`All kinds`}
        value={filter.kind ?? ""}
        onChange={(kind) => onFilter({ ...filter, kind: (kind || null) as TransactionKind | null })}
        options={EDITABLE_TRANSACTION_KINDS.map((kind) => ({
          value: kind,
          label: transactionLabel(i18n, kind),
        }))}
      />

      <Choice
        label={t`Year`}
        placeholder={t`All years`}
        value={filter.from?.slice(0, 4) ?? ""}
        onChange={(year) =>
          onFilter({
            ...filter,
            from: year ? `${year}-01-01` : null,
            to: year ? `${year}-12-31` : null,
          })
        }
        options={years.map((year) => ({ value: year, label: year }))}
      />
    </>
  );
}
