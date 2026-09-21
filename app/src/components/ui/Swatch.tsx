import { slotVar } from "../../lib/plot";

/**
 * The palette square that names a colour: a taxonomy node in a pill, a series in a legend,
 * a row in a bar chart. Without a slot it inherits the `--slot` of whatever set it, so a row
 * paints its own marker.
 */
export function Swatch({ slot, className }: { slot?: number; className?: string }) {
  return <i className={`sw${className ? ` ${className}` : ""}`} style={slotVar(slot)} aria-hidden="true" />;
}
