import { useLingui } from "@lingui/react/macro";
import { needsCostQuery, resolveColumnIds } from "../../components/domain/positionColumns";
import {
  usePortfolio,
  usePositionReturns,
  usePositions,
  usePositionsCostBasis,
  useSecurities,
} from "../../lib/queries";
import type { DateString, PeriodRange, WatchRow } from "../../lib/types";
import { useWatchColumns } from "./columns";

/** Position figures joined from the positions screen's queries, not recomputed. */
export function useWatchContext(date: DateString, range: PeriodRange | undefined, stored: string[]) {
  const { i18n } = useLingui();
  const positions = usePositions(date);
  const returns = usePositionReturns(range);
  const portfolio = usePortfolio();
  const securities = useSecurities();
  const allColumns = useWatchColumns();
  const method = portfolio.data?.cost_basis_method;
  // Only a purchase figure under the *other* method is worth the host's second holdings pass.
  const shownColumns = resolveColumnIds(stored, method);
  const costs = usePositionsCostBasis(needsCostQuery(shownColumns, method) ? date : null);

  const currency = portfolio.data?.base_currency ?? "";
  const positionOf = new Map((positions.data?.rows ?? []).map((r) => [r.security_id, r]));
  const returnOf = new Map((returns.data ?? []).map((r) => [r.security_id, r]));
  const securityOf = new Map((securities.data ?? []).map((s) => [s.id, s]));
  const costOf = new Map((costs.data?.rows ?? []).map((r) => [r.security_id, r]));
  const ctx = (row: WatchRow) => ({
    currency,
    i18n,
    from: range?.from,
    position: positionOf.get(row.security_id),
    period: returnOf.get(row.security_id),
    security: securityOf.get(row.security_id),
    cost: costOf.get(row.security_id),
    own: method,
  });

  return { currency, allColumns, shownColumns, securities: securities.data, ctx };
}
