/** A placeholder in the value's shape, so nothing moves when data arrives; silent to screen readers. */
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
