import { useLingui } from "@lingui/react/macro";
import { QUOTE_WINDOW_DAYS, useQuoteWindow } from "../../lib/queries";
import { Spark } from "../charts";

/** The last 90 days of one instrument as a sparkline; it shows shape, not exact daily values. */
export function QuoteSpark({ securityId, symbol }: { securityId: string; symbol: string }) {
  const { t } = useLingui();
  const quotes = useQuoteWindow(securityId);

  if (!quotes.data || quotes.data.length < 2) return <span className="dim">—</span>;
  return (
    <Spark
      values={quotes.data.map((q) => Number(q.close))}
      label={t`${symbol} over ${QUOTE_WINDOW_DAYS} days`}
    />
  );
}
