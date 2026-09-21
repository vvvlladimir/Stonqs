import { useMutation, useQuery } from "@tanstack/react-query";
import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { api } from "./api";
import { keys, useInvalidate } from "./queries";
import type { BuiltinPreset, DateString, PeriodRange, PeriodSpec, UserPeriod } from "./types";

/**
 * The period axis. Dates come from the core (`period_ranges`), which resolves the seven
 * shipped presets and whatever the user added — see ADR-0018 and ADR-0026. The frontend owns
 * the wording of the shipped seven and nothing else; a user period carries its own name.
 */

/** A period's identity on the wire: a shipped preset's code or a user period's id. */
export type PeriodId = string;

export const BUILTIN_LABELS: Record<BuiltinPreset, MessageDescriptor> = {
  ONE_MONTH: msg`1 month`,
  THREE_MONTHS: msg`3 months`,
  YTD: msg`Year to date`,
  ONE_YEAR: msg`1 year`,
  THREE_YEARS: msg`3 years`,
  FIVE_YEARS: msg`5 years`,
  SINCE_INCEPTION: msg`All time`,
};

/** Short labels: the segmented control shows these, the exact dates live in the page header. */
export const BUILTIN_SHORT: Record<BuiltinPreset, MessageDescriptor> = {
  ONE_MONTH: msg`1M`,
  THREE_MONTHS: msg`3M`,
  YTD: msg`YTD`,
  ONE_YEAR: msg`1Y`,
  THREE_YEARS: msg`3Y`,
  FIVE_YEARS: msg`5Y`,
  SINCE_INCEPTION: msg`All`,
};

/** Shortest first, matching the order the host returns ranges in. */
export const BUILTIN_PRESETS: BuiltinPreset[] = [
  "ONE_MONTH",
  "THREE_MONTHS",
  "YTD",
  "ONE_YEAR",
  "THREE_YEARS",
  "FIVE_YEARS",
  "SINCE_INCEPTION",
];

function isBuiltin(value: unknown): value is BuiltinPreset {
  return typeof value === "string" && value in BUILTIN_LABELS;
}

/** Units a relative window counts back in, in the order the editor offers them. */
export const UNIT_LABELS = {
  DAY: msg`days`,
  WEEK: msg`weeks`,
  MONTH: msg`months`,
  QUARTER: msg`quarters`,
  YEAR: msg`years`,
} as const;

/**
 * A period's name. The user's own wording wins; a shipped preset falls back to the catalog,
 * and an id that is neither is shown as-is rather than as a blank button.
 */
export function periodLabel(i18n: I18n, range: Pick<PeriodRange, "id" | "name">, short = true): string {
  if (range.name) return range.name;
  const table = short ? BUILTIN_SHORT : BUILTIN_LABELS;
  return isBuiltin(range.id) ? i18n._(table[range.id]) : range.id;
}

export function usePeriodRanges(asOf: DateString) {
  return useQuery({ queryKey: keys.periods(asOf), queryFn: () => api.periodRanges(asOf) });
}

/** The editable half of the axis: the user's periods and the presets they hid. */
export function usePeriodSettings() {
  return useQuery({ queryKey: keys.periodSettings(), queryFn: api.periodsGet });
}

/**
 * Every period mutation invalidates the resolved strip as well as the editor's own list,
 * because adding a period changes what every screen may pick.
 */
export function usePeriodEdit() {
  const invalidate = useInvalidate();
  const done = () => invalidate(keys.periodSettings(), keys.periods());

  return {
    save: useMutation({ mutationFn: (period: UserPeriod) => api.periodSave(period), onSuccess: done }),
    remove: useMutation({ mutationFn: (id: PeriodId) => api.periodDelete(id), onSuccess: done }),
    restore: useMutation({ mutationFn: () => api.periodsRestore(), onSuccess: done }),
  };
}

/** A new period's id. Stable and opaque: the name is free to change afterwards. */
export function newPeriodId(): PeriodId {
  return `p-${crypto.randomUUID().slice(0, 8)}`;
}

/** The spec a freshly opened editor starts from. */
export const DEFAULT_SPEC: PeriodSpec = { kind: "RELATIVE", unit: "MONTH", count: 6 };

/**
 * Returns the requested range or the nearest available one. A stored id that no longer
 * resolves — a period the user deleted, or one this portfolio's history cannot cover — falls
 * back to the first offered range rather than leaving the screen without a period.
 */
export function pickRange(ranges: PeriodRange[] | undefined, id: PeriodId): PeriodRange | undefined {
  return ranges?.find((r) => r.id === id) ?? ranges?.[0];
}
