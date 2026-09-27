import { slotVar } from "../../lib/plot";

/** A palette square; without a slot it inherits the `--slot` of its parent. */
export function Swatch({ slot, className }: { slot?: number; className?: string }) {
  return <i className={`sw${className ? ` ${className}` : ""}`} style={slotVar(slot)} aria-hidden="true" />;
}
