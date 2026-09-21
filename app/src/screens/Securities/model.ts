import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { SecurityInput, SecurityRow } from "../../lib/types";

export const EMPTY: SecurityInput = {
  id: null,
  symbol: "",
  name: "",
  currency: "USD",
  kind: "STOCK",
  isin: null,
  // Filled with the host's default source when the form opens (`quote_providers` lists it first).
  data_source: null,
  data_symbol: null,
  quantity_step: null,
  wkn: null,
  note: null,
  attributes: {},
};

/** The fixed slices of the directory, rendered as segmented controls. */
export type Cut = "all" | "no_venue" | "unidentified";

export function cuts(i18n: I18n) {
  return [
    { value: "all" as const, label: i18n._(msg`All`) },
    { value: "no_venue" as const, label: i18n._(msg`No venue`) },
    { value: "unidentified" as const, label: i18n._(msg`Not identified`) },
  ];
}

/**
 * Two different faults, so two slices: a row without a MIC is priced from an unstated venue,
 * while an unidentified one carries an ISIN where the provider expects a ticker.
 */
export function inCut(row: SecurityRow, cut: Cut): boolean {
  if (cut === "no_venue") return row.mic === null;
  if (cut === "unidentified") return row.needs_lookup;
  return true;
}

/** Explain a directory/quote currency mismatch before valuation surprises the user. */
export function currencyMismatch(row: SecurityRow): boolean {
  return row.quote_currency !== null && row.quote_currency !== row.currency;
}
