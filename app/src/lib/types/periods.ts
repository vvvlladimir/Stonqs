/** The period axis, and the series measured over it. */

import type { DateString, MoneyString } from "./primitives";
export interface ValueSeries {
  base_currency: string;
  dates: DateString[];
  total_value_base: MoneyString[];
  external_flow_base: MoneyString[];
}

/** Codes of the seven shipped presets; a user period's id is any other string. */
export type BuiltinPreset =
  "ONE_MONTH" | "THREE_MONTHS" | "YTD" | "ONE_YEAR" | "THREE_YEARS" | "FIVE_YEARS" | "SINCE_INCEPTION";

export interface PeriodRange {
  /** A shipped preset's code, or a user period's id. */
  id: string;
  /** The user's own wording; `null` for a shipped preset, whose label the frontend owns. */
  name: string | null;
  from: DateString;
  to: DateString;
}

/** Calendar unit a relative window counts back in. */
export type PeriodUnit = "DAY" | "WEEK" | "MONTH" | "QUARTER" | "YEAR";

/**
 * How a user period finds its dates. Tagged, so the two kinds differ by a field, not a shape.
 * A `FIXED` window with `to: null` is open-ended: it runs to the reporting date.
 */
export type PeriodSpec =
  | { kind: "RELATIVE"; unit: PeriodUnit; count: number }
  | { kind: "FIXED"; from: DateString; to: DateString | null };

export interface UserPeriod {
  id: string;
  name: string;
  spec: PeriodSpec;
}

/** What the period editor edits: the user's own periods and the presets they hid. */
export interface PeriodSettings {
  periods: UserPeriod[];
  hidden_presets: string[];
}

/** Daily statistical series with parallel date and value arrays. */
export interface StatSeries {
  dates: DateString[];
  values: number[];
}

/** Unit growth series; `1.12` represents a 12% gain. */
export interface GrowthSeries {
  dates: DateString[];
  values: MoneyString[];
}
