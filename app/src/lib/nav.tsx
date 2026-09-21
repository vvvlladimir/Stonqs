import { createContext, useContext, type ReactNode } from "react";

/** Navigation context supports focused cross-screen transitions. */

export type ScreenId =
  | "dashboard"
  | "positions"
  | "transactions"
  | "accounts"
  | "plans"
  | "alerts"
  | "securities"
  | "watchlist"
  | "performance"
  | "trades"
  | "risk"
  | "allocation"
  | "rebalance"
  | "income"
  | "import"
  | "reports"
  | "settings";

interface Nav {
  /** Where the user is. Read by the assistant panel, which is above the screens rather than one
   *  of them and otherwise has no way to know what is on the other side of it. */
  screen: ScreenId;
  go: (screen: ScreenId, focus?: string) => void;
}

const NavContext = createContext<Nav>({ screen: "dashboard", go: () => {} });

export function NavProvider({ value, children }: { value: Nav; children: ReactNode }) {
  return <NavContext.Provider value={value}>{children}</NavContext.Provider>;
}

export function useNav(): Nav {
  return useContext(NavContext);
}
