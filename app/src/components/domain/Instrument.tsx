import type { ReactNode } from "react";
import { useLingui } from "@lingui/react/macro";
import { securityKindLabel } from "../../lib/kinds";
import { SecurityLink } from "./SecurityCardProvider";
import type { SecurityKind } from "../../lib/types";

/**
 * Security row: monogram, name, ticker and kind. One component shared by positions,
 * the security directory and the transaction log, so the same instrument looks the same everywhere.
 *
 */
export function Logo({ symbol, name, icon }: { symbol: string; name?: string; icon?: ReactNode }) {
  const source = symbol || name || "";
  const mono = source
    .replace(/[^\p{L}\p{N}]/gu, "")
    .slice(0, 2)
    .toUpperCase();
  return (
    <span className="logo" aria-hidden="true">
      {icon ?? mono}
    </span>
  );
}

interface Props {
  symbol: string;
  name: string;
  kind?: SecurityKind;
  /** Ticker under the name; turned off where it has its own column. */
  ticker?: boolean;
  /** With an id the whole row opens the instrument's card; without one it is plain text. */
  id?: string | null;
}

export function Instrument({ symbol, name, kind, ticker = true, id }: Props) {
  const { i18n } = useLingui();
  return (
    <SecurityLink id={id} className="instr">
      <Logo symbol={symbol} name={name} />
      <div>
        {/* Preserve the full name in the native tooltip when the cell truncates it. */}
        <div className="nm" title={name || symbol}>
          {name || symbol}
        </div>
        {(ticker || kind) && (
          <div className="sub">
            {ticker && <span>{symbol}</span>}
            {kind && <span className="tag">{securityKindLabel(i18n, kind)}</span>}
          </div>
        )}
      </div>
    </SecurityLink>
  );
}
