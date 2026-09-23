import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

import { today } from "./api";
import type { DateString } from "./types";

/**
 * The date every reading screen is answered for — a lens over time, beside the scope's lens
 * over accounts. Session state on purpose: a date restored from disk would open the app in the
 * past without anybody asking for it, and every figure on screen would look broken.
 */
interface AsOf {
  date: DateString;
  isToday: boolean;
  set: (date: DateString) => void;
  reset: () => void;
}

const AsOfContext = createContext<AsOf | null>(null);

export function AsOfProvider({ children }: { children: ReactNode }) {
  // `null` is "today", resolved on read, so a window left open overnight moves with the clock.
  const [picked, setPicked] = useState<DateString | null>(null);
  const now = today();
  const value = useMemo<AsOf>(
    () => ({
      date: picked ?? now,
      isToday: picked === null || picked === now,
      set: (date) => setPicked(date === now ? null : date),
      reset: () => setPicked(null),
    }),
    [picked, now],
  );
  return <AsOfContext.Provider value={value}>{children}</AsOfContext.Provider>;
}

export function useAsOf(): AsOf {
  const value = useContext(AsOfContext);
  // Outside the provider the app is its own history: today, and nothing to switch.
  const now = today();
  return value ?? { date: now, isToday: true, set: () => {}, reset: () => {} };
}
