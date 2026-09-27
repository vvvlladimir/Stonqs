import { Plural, Trans } from "@lingui/react/macro";

import { formatDay } from "../../lib/format";
import { useNav } from "../../lib/nav";
import { useLedgerGaps, useSecurities } from "../../lib/queries";
import type { QuantityGap } from "../../lib/types";
import { Banner } from "../ui";

/** Holes are bridged at the sale's price (ADR-0089), so this says the figures are estimates. */
export function LedgerGapsBanner() {
  const gaps = useLedgerGaps();
  const securities = useSecurities();
  const { go } = useNav();
  if (!gaps.data || gaps.data.length === 0) return null;

  // One entry per instrument, in the order the first hole appears.
  const byInstrument = new Map<string, QuantityGap[]>();
  for (const gap of gaps.data) {
    const list = byInstrument.get(gap.security_id);
    if (list) list.push(gap);
    else byInstrument.set(gap.security_id, [gap]);
  }
  const symbolOf = (id: string) => securities.data?.find((s) => s.id === id)?.symbol ?? "…";
  const [firstId, firstGaps] = [...byInstrument.entries()][0];
  const symbol = symbolOf(firstId);
  const since = formatDay(firstGaps[0].date);
  const count = firstGaps.length;
  const symbols = [...byInstrument.keys()].map(symbolOf).join(", ");

  return (
    <div className="timelens">
      <Banner
        tone="bad"
        action={
          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => go("transactions", byInstrument.size === 1 ? symbol : undefined)}
          >
            <Trans>Show operations</Trans>
          </button>
        }
      >
        {byInstrument.size === 1 ? (
          <Trans>
            {symbol} is sold without having been bought: <Plural value={count} one="# sale" other="# sales" />{" "}
            from {since} take more than the operations ever received. Its figures are estimated until the
            purchase or the incoming transfer is added — as a new transaction, or by importing the file again
            with that row mapped.
          </Trans>
        ) : (
          <Trans>
            {symbols} are sold without having been bought. Their figures are estimated until the purchases or
            incoming transfers are added — as new transactions, or by importing the files again with those
            rows mapped.
          </Trans>
        )}
      </Banner>
    </div>
  );
}
