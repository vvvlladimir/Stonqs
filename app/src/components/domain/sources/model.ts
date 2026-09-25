import type { CustomSource, MarketSourceRow } from "../../../lib/types";

/**
 * What a set of sources still lacks before the app can price a portfolio by itself.
 *
 * Two answers are needed and neither substitutes for the other: something that publishes
 * prices, and something that publishes exchange rates — a portfolio held in one currency and
 * reported in another is the ordinary case, and a missing rate is `MissingMarketData`, not a
 * smaller number. Everything else a catalogue offers (venues, search, inflation) makes the app
 * better informed, never able or unable to value a holding.
 *
 * A source with a required key it does not have is counted as absent: it is switched on into
 * the state that reads as broken.
 */
export interface SourceNeeds {
  quotes: boolean;
  rates: boolean;
  /** Both answered — the minimum the sources dialog asks for before it will confirm. */
  ok: boolean;
}

/**
 * What is picked answers this, not what is being asked: confirming is what turns one into the
 * other. `switched` is `AppSettings::market_sources`, which is the only place a source of the
 * user's own carries a switch — the catalogue's rows carry their own, and a custom source with
 * no entry there is on.
 */
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
