/** Net worth and the valuations behind it. */

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { api } from "../api";
import type { DateString } from "../types";
import { keys } from "./keys";

/** Not scoped: assets are not accounts, so the reading is always the whole portfolio. The tax
 *  rate is part of the key — it is an assumption, and two of them are two answers. */
export function useNetWorth(date: DateString, taxRate?: string) {
  const rate = taxRate?.trim() === "" ? undefined : taxRate;
  return useQuery({
    queryKey: keys.netWorth(date, rate),
    queryFn: () => api.netWorth(date, rate),
    // A changed rate is the same reading with one more answer on it, so the screen keeps the
    // figures it is showing rather than blanking to a spinner while the new one arrives.
    placeholderData: keepPreviousData,
  });
}

export function useNetWorthSeries(from: DateString, to: DateString) {
  return useQuery({
    queryKey: keys.netWorthSeries(from, to),
    queryFn: () => api.netWorthSeries(from, to),
  });
}

/** Disabled without an asset: the history of nothing is not a question. */
export function useAssetValues(assetId: string | null) {
  return useQuery({
    queryKey: keys.assetValues(assetId ?? undefined),
    queryFn: () => api.assetValues(assetId!),
    enabled: assetId !== null,
  });
}
