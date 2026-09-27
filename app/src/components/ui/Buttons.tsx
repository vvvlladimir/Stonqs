import type { ReactNode } from "react";

/** A visible button cluster; row actions belong to `ListRow`'s `actions`. */
export function Buttons({ children }: { children: ReactNode }) {
  return <div className="btns">{children}</div>;
}
