import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";

export type Tab = "gains" | "dividends" | "charges";

export function tabs(i18n: I18n) {
  return [
    { value: "gains" as const, label: i18n._(msg`Realized`) },
    { value: "dividends" as const, label: i18n._(msg`Dividends`) },
    { value: "charges" as const, label: i18n._(msg`Costs`) },
  ];
}

/** One CSV file the host can produce. `section` is the whole request (see `report_export`). */
export interface Export {
  section: string;
  label: string;
}

/** Every table on a tab is exportable, so the file matches what is on screen. */
export function exportsOf(i18n: I18n): Record<Tab, Export[]> {
  return {
    gains: [
      { section: "gains.detail", label: i18n._(msg`Transactions — every row`) },
      { section: "gains.security", label: i18n._(msg`Totals by instrument`) },
      { section: "gains.year", label: i18n._(msg`Totals by year`) },
    ],
    dividends: [
      { section: "dividends.detail", label: i18n._(msg`Payments — every row`) },
      { section: "dividends.security", label: i18n._(msg`Totals by instrument`) },
      { section: "dividends.year", label: i18n._(msg`Totals by year`) },
    ],
    charges: [
      { section: "charges.detail", label: i18n._(msg`Transactions — every row`) },
      { section: "charges.kind", label: i18n._(msg`Totals by kind`) },
      { section: "charges.account", label: i18n._(msg`Totals by account`) },
      { section: "charges.year", label: i18n._(msg`Totals by year`) },
    ],
  };
}
