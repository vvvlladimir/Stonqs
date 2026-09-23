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

/** Last day of the month before this one — a point in time, not a period (ADR-0018). */
export function endOfPreviousMonth(date: DateString): DateString {
  const [year, month] = date.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, 0)).toISOString().slice(0, 10);
}

/** 31 December of the year before this one. */
export function endOfPreviousYear(date: DateString): DateString {
  const year = Number(date.slice(0, 4));
  return `${year - 1}-12-31`;
}
