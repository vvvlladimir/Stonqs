import type { CustomSource, MarketSourceRow } from "../../../lib/types";

/** Something for prices and something for rates are both needed; a source missing its required key counts as absent. */
export interface SourceNeeds {
  quotes: boolean;
  rates: boolean;
  /** Both answered — the minimum the sources dialog asks for before it will confirm. */
  ok: boolean;
}

/** Answered by what is picked, not what is asked; a custom source with no switch entry is on. */
export function sourceNeeds(
  rows: MarketSourceRow[],
  custom: CustomSource[] = [],
  switched: Record<string, boolean> = {},
): SourceNeeds {
  const usable = rows.filter((row) => row.wanted && (row.key !== "required" || row.has_key));
  const mine = custom.filter((source) => switched[source.id] ?? true);
  const quotes =
    usable.some((row) => row.capabilities.includes("quotes")) ||
    mine.some((source) => source.role === "quotes");
  const rates =
    usable.some((row) => row.capabilities.includes("fx_rates")) ||
    mine.some((source) => source.role === "fx");
  return { quotes, rates, ok: quotes && rates };
}
