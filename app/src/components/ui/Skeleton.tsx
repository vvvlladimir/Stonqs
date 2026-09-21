/**
 * A placeholder shaped like the value that will replace it, so a panel is laid out once and
 * nothing moves when the query answers. It says nothing to a screen reader: the region around
 * it is what announces that data is on its way.
 */
export function Skeleton({ w, h, className }: { w?: string; h?: string; className?: string }) {
  return (
    <span
      className={className ? `skel ${className}` : "skel"}
      style={{ width: w, height: h }}
      aria-hidden="true"
    />
  );
}

/** A stack of lines standing in for a list: one placeholder per row that is coming. */
export function SkeletonRows({ rows, h = "1.1rem" }: { rows: number; h?: string }) {
  return (
    <div className="skel-rows">
      {Array.from({ length: rows }, (_, i) => (
        <Skeleton key={i} h={h} />
      ))}
    </div>
  );
}
