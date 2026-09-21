import type { ReactNode } from "react";

/**
 * A visible cluster of buttons — a panel's own controls, the actions of a card.
 * Row actions are not this: they belong to `ListRow`'s `actions`, which hides them until hover.
 */
export function Buttons({ children }: { children: ReactNode }) {
  return <div className="btns">{children}</div>;
}
