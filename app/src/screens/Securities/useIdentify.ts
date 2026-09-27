import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { affects, useInvalidate } from "../../lib/queries";
import type { SecurityRow } from "../../lib/types";

/** Resolves imported ISIN placeholders to a provider symbol, one row or all of them. */
export function useIdentify() {
  const invalidate = useInvalidate();
  const identify = useMutation({
    mutationFn: api.securityIdentify,
    onSuccess: () => invalidate(...affects.securities),
  });

  /** Sequential, to stay under the provider's rate limit. */
  const identifyAll = async (rows: SecurityRow[]) => {
    for (const row of rows) {
      try {
        await identify.mutateAsync(row.id);
      } catch {
        // One failed lookup must not stop the remaining rows.
      }
    }
  };

  return { identify, identifyAll };
}
