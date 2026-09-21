import type { ReactNode } from "react";

/** Tags and badges describe state; they are not interactive controls. */
export function Tag({ warn, children }: { warn?: boolean; children: ReactNode }) {
  return <span className={`tag${warn ? " tag--warn" : ""}`}>{children}</span>;
}

export type BadgeTone = "neutral" | "in" | "out" | "warn" | "info";

export function Badge({ tone = "neutral", children }: { tone?: BadgeTone; children: ReactNode }) {
  return <span className={`badge${tone === "neutral" ? "" : ` badge--${tone}`}`}>{children}</span>;
}
