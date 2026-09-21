/** Watchlists and the rows they show. */

import type { AlertDirection } from "./alerts";
import type { DateString, MoneyString } from "./primitives";
import type { SecurityKind } from "./securities";
/** A named list of instruments, held or not, in the user's order. */
export interface Watchlist {
  id: string;
  name: string;
  security_ids: string[];
}

export interface WatchlistInput {
  id?: string | null;
  name: string;
  security_ids: string[];
}

/** The trigger level the price is closest to. */
export interface NearestLevel {
  alert_id: string;
  level: MoneyString;
  currency: string;
  direction: AlertDirection;
  /** `level / price − 1`: positive when the level is above the price. */
  distance: MoneyString;
}

/** One instrument's own price facts, in its quote currency; `null` without a close on file. */
export interface WatchRow {
  security_id: string;
  symbol: string;
  name: string;
  kind: SecurityKind;
  currency: string | null;
  price: MoneyString | null;
  price_date: DateString | null;
  previous_price: MoneyString | null;
  day_change: MoneyString | null;
  /** The close the period's move starts at; later than the period for a younger instrument. */
  period_start: DateString | null;
  start_price: MoneyString | null;
  period_return: MoneyString | null;
  low: MoneyString | null;
  high: MoneyString | null;
  /** 0 at the period's low, 1 at its high. */
  range_position: MoneyString | null;
  ath_price: MoneyString | null;
  ath_date: DateString | null;
  ath_distance: MoneyString | null;
  /** Dividends per share the provider reported over the last year, and their yield on the price. */
  dividend_year: MoneyString | null;
  dividend_yield: MoneyString | null;
  dividend_last: DateString | null;
  nearest_level: NearestLevel | null;
}
