import { plural } from "@lingui/core/macro";
import { Trans } from "@lingui/react/macro";
import { Money, Progress } from "../../components/ui";
import { formatDay, formatPercent } from "../../lib/format";
import type { Asset, AssetHolding } from "../../lib/types";

/** Under a thing owned: what it leaves after the debt secured on it. Both figures are in the
 *  base currency — a flat in one currency can carry a debt in another, and only the base has
 *  both. */
export function EquityLine({ holding, base }: { holding: AssetHolding; base: string }) {
  if (holding.equity_base === null || holding.secured_debt_base === null) return null;
  return (
    <span className="dim">
      <Trans>
        <Money value={holding.equity_base} currency={base} /> yours, after{" "}
        <Money value={holding.secured_debt_base} currency={base} /> owed on it
      </Trans>
    </span>
  );
}

/**
 * Under a debt: how much of it is gone and where it ends. The track is measured from the first
 * figure ever written, not from the schedule — progress has to be something that happened.
 */
export function PayoffTrack({ asset, holding }: { asset: Asset; holding: AssetHolding }) {
  const payoff = holding.payoff;
  if (!payoff) return null;
  const rate = asset.schedule ? Number(asset.schedule.rate) * 100 : null;
  // Progress is measured between two figures. With one, there is no track to draw — an empty bar
  // reads as "nothing repaid", which is a claim nobody made.
  const measured = payoff.paid_share !== null;

  const end =
    payoff.payoff_on !== null ? (
      <Trans>gone by {formatDay(payoff.payoff_on)}</Trans>
    ) : (
      // The payment does not cover the month's interest, so no date would be honest.
      <Trans>this payment never clears it</Trans>
    );

  if (!measured)
    return (
      <span className="dim">
        {rate !== null && <>{String(rate)}% · </>}
        {payoff.months_left !== null && (
          <>{plural(payoff.months_left, { one: "# month left", other: "# months left" })} · </>
        )}
        {end}
      </span>
    );

  return (
    <Progress
      size="sm"
      share={payoff.paid_share}
      barTone={Number(payoff.paid_share) < 0 ? "neg" : undefined}
      legend={{
        left: (
          <>
            {payoff.paid_share !== null && <>{formatPercent(payoff.paid_share)} </>}
            <Trans>paid off</Trans>
            {payoff.months_left !== null && (
              <>
                {" · "}
                {plural(payoff.months_left, { one: "# month left", other: "# months left" })}
              </>
            )}
          </>
        ),
        right: (
          <span className="dim">
            {rate !== null && <>{String(rate)}% · </>}
            {end}
          </span>
        ),
      }}
      note={
        payoff.interest_ahead !== null ? (
          <span className="dim">
            <Trans>
              <Money value={payoff.interest_ahead} currency={asset.currency} /> of interest still to pay, and
              the contract ends{" "}
              {payoff.ends_on !== null ? formatDay(payoff.ends_on) : formatDay(payoff.payoff_on!)}
            </Trans>
          </span>
        ) : undefined
      }
    />
  );
}
