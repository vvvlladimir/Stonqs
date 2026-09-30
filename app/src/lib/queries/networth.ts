/** Net worth and the valuations behind it. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import type { DateString } from "../types";
import { keys } from "./keys";

/** Not scoped: assets are not accounts, so the reading is always the whole portfolio. */
export function useNetWorth(date: DateString) {
  return useQuery({ queryKey: keys.netWorth(date), queryFn: () => api.netWorth(date) });
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
