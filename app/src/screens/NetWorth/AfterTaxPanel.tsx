import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Facts, Fact, Money, Panel, PercentInput } from "../../components/ui";
import type { AfterTax } from "../../lib/types";
import { taxRateOf } from "./model";

/**
 * Net worth after the tax selling would cost. A second view beside the total, never instead of
 * it, and exact only where the app has a purchase price — which a flat never has (ADR-0093).
 */
export function AfterTaxPanel({
  rate,
  onRate,
  view,
  currency,
}: {
  /** Percent as typed; empty asks for no view. */
  rate: string;
  onRate: (rate: string) => void;
  view: AfterTax | null;
  currency: string;
}) {
  const { t } = useLingui();
  // What is being typed is local: the stated rate is a saved setting and a query of its own, so
  // applying it on every keystroke would reload the screen under the cursor. A rate that changed
  // elsewhere (another profile, another window) is taken over during the render that brings it.
  const [draft, setDraft] = useState(rate);
  const [shown, setShown] = useState(rate);
  if (shown !== rate) {
    setShown(rate);
    setDraft(rate);
  }
  const { invalid } = taxRateOf(draft);

  return (
    <Panel
      title={t`After tax`}
      info={t`Your own rate over the portfolio's unrealized gain. It covers nothing that has no purchase price.`}
      tools={
        <PercentInput
          label={t`Tax rate on a gain, % `}
          value={draft}
          onChange={setDraft}
          // A figure outside 0–100% is neither applied nor saved: the field says so, and the
          // reading keeps whatever rate it had.
          onCommit={(typed) => !taxRateOf(typed).invalid && onRate(typed)}
          invalid={invalid}
          tip={invalid ? t`A tax rate is between 0 and 100%.` : undefined}
        />
      }
    >
      {view === null ? (
        <p className="muted">
          <Trans>
            State a rate and this says what would be left if you sold the portfolio today. Nothing else in the
            app reads it.
          </Trans>
        </p>
      ) : (
        <>
          <Facts>
            <Fact
              label={<Trans>Gain not yet taxed</Trans>}
              value={<Money value={view.taxable_gain_base} currency={currency} />}
            />
            <Fact
              label={<Trans>Tax if sold today</Trans>}
              value={<Money value={view.tax_base} currency={currency} />}
            />
            <Fact
              label={<Trans>Net worth after it</Trans>}
              value={<Money value={view.net_after_tax_base} currency={currency} />}
            />
          </Facts>
          {Number(view.outside_base) > 0 && (
            <p className="muted">
              <Trans>
                <Money value={view.outside_base} currency={currency} /> of things owned is outside this: they
                have no purchase price, so there is no gain here to tax.
              </Trans>
            </p>
          )}
        </>
      )}
    </Panel>
  );
}
