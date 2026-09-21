import { CH, linearScale, path } from "../../lib/plot";

/** Compact trend line for dense table rows. */
interface Props {
  values: number[];
  /** Accessible description for the trend image. */
  label: string;
}

const WIDTH = 66;
const HEIGHT = CH.h.spark;
const INSET = 2; // Keep the stroke inside the viewBox.

export function Spark({ values, label }: Props) {
  if (values.length < 2) return <span className="muted">—</span>;

  const min = Math.min(...values);
  const max = Math.max(...values);
  const x = linearScale([0, values.length - 1], [INSET, WIDTH - INSET]);
  // A constant series has no range, so center it instead of using a degenerate scale.
  const y = max === min ? () => HEIGHT / 2 : linearScale([min, max], [HEIGHT - INSET, INSET]);

  const tone =
    values[values.length - 1] > values[0] ? "pos" : values[values.length - 1] < values[0] ? "neg" : "";

  return (
    <svg className={`spark ${tone}`} viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img" aria-label={label}>
      <path d={path(values.map((v, i) => [x(i), y(v)]))} />
    </svg>
  );
}
