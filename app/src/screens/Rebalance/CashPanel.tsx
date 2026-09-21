import { Trans, useLingui } from "@lingui/react/macro";
import { Chip, Chips, Money, Num, Panel } from "../../components/ui";
import { formatDecimal } from "../../lib/format";
import { PRESETS } from "./model";

/** New money is added to the total before targets are derived, so it changes the whole plan. */
export function CashPanel({
  cash,
  onCash,
  total,
  currency,
}: {
  cash: string;
  onCash: (cash: string) => void;
  total: string | undefined;
  currency: string;
}) {
  const { t } = useLingui();
  return (
    <Panel
      title={t`New money`}
      info={t`Money to invest on top of the portfolio; with "Buy only" the drift is closed by buying, never by selling.`}
    >
      <div className="cashline">
        <label className="field cashline__field">
          <span className="field__label">
            <Trans>How much to add</Trans>
          </span>
          <input inputMode="decimal" value={cash} placeholder="0" onChange={(e) => onCash(e.target.value)} />
        </label>
        <Chips>
          {PRESETS.map((value) => (
            <Chip key={value} active={cash.trim() === value} onClick={() => onCash(value)}>
              <Num>{formatDecimal(value, { digits: 0 })}</Num>
            </Chip>
          ))}
          <Chip active={cash.trim() === ""} onClick={() => onCash("")}>
            <Trans>Reset</Trans>
          </Chip>
        </Chips>
        <span className="spacer" />
        <div className="cashline__after">
          <span className="panel__note">
            <Trans>Portfolio after</Trans>
          </span>
          <b>{total ? <Money value={total} currency={currency} digits={0} /> : "…"}</b>
        </div>
      </div>
    </Panel>
  );
}
