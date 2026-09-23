import type { ReactNode } from "react";
import type { I18n, MessageDescriptor } from "@lingui/core";
import type { Icon } from "@phosphor-icons/react";
import { periodLabel, pickRange, usePeriodRanges, type PeriodId } from "../../../lib/periods";
import { useRisk, useTargets } from "../../../lib/queries";
import type { Source } from "../../../lib/api";
import type { Widget } from "../../../lib/uiState";
import { msg } from "@lingui/core/macro";
import type {
  ChargeSummary,
  DataScope,
  DateString,
  IncomeSummary,
  MoneyString,
  PeriodRange,
  PeriodSummary,
  PortfolioValuation,
  ScopeKind,
} from "../../../lib/types";

export interface MetricCtx {
  date: DateString;
  /** Widget period, resolved to dates by `period_ranges`. */
  period: PeriodId;
  /** Widget data source; absent means the one the picker holds. */
  source?: Source;
  /** What the line under the value says: empty is the metric's own note. */
  foot?: string;
}

export interface MetricDef {
  label: MessageDescriptor;
  /** Description shown beside the value. */
  tip: MessageDescriptor;
  /** Whether the metric needs a period in the widget header. */
  periodic?: boolean;
  Render: (ctx: MetricCtx) => ReactNode;
}

/** Settings supported by the shared configuration dialog. */
export type Field =
  | "title"
  | "source"
  | "metric"
  | "ratio"
  | "fire"
  | "period"
  | "foot"
  | "taxonomy"
  | "count"
  | "benchmark"
  | "inflation"
  | "target"
  | "watchlist"
  | "goal"
  | "limit"
  | "prompt"
  | "refresh"
  | "length"
  | "model";

export interface WidgetProps {
  widget: Widget;
  date: DateString;
  /** Dashboard period, optionally overridden by `cfg.period`. */
  period: PeriodId;
}

const SCOPE_KINDS: ScopeKind[] = ["PORTFOLIO", "GROUP", "ACCOUNT", "ACCOUNT_WITH_CASH"];

/**
 * The data source a widget reads from: its own, or — the usual case — the one the picker
 * holds, which is what `undefined` asks the host for. A board can therefore carry one tile per
 * account without the screen around it changing.
 */
export function sourceOf(widget: Widget): DataScope | undefined {
  const own = widget.cfg.source as Partial<DataScope> | undefined;
  if (!own || typeof own !== "object") return undefined;
  const kind = SCOPE_KINDS.find((k) => k === own.kind);
  if (!kind) return undefined;
  return { kind, id: typeof own.id === "string" ? own.id : null };
}

/**
 * Which provider and model write a tile, when it names its own. A model belongs to the provider
 * it was picked from, so one without the other is only ever the provider: its newest model.
 */
export function modelOf(widget: Widget): { provider: string | null; model: string | null } {
  const text = (key: string) => {
    const value = widget.cfg[key];
    return typeof value === "string" && value.trim() !== "" ? value : null;
  };
  const provider = text("provider");
  return { provider, model: provider ? text("model") : null };
}

/**
 * The figures a ratio may be built from: balances read on the date, and changes read over the
 * period. Both come from queries the dashboard already makes, so a ratio tile beside a metric
 * tile costs nothing extra.
 */
export const RATIO_TERMS: Record<
  string,
  { label: MessageDescriptor; periodic?: boolean; pick: (data: RatioData) => MoneyString | undefined }
> = {
  value: { label: msg`Value`, pick: (d) => d.valuation?.total_value_base },
  securities: { label: msg`Securities`, pick: (d) => d.valuation?.securities_value_base },
  cash: { label: msg`Cash`, pick: (d) => d.valuation?.cash_base },
  invested: { label: msg`Cost basis`, pick: (d) => d.valuation?.cost_basis_base },
  unrealized: { label: msg`Unrealized P/L`, pick: (d) => d.valuation?.unrealized_pnl_base },
  realized: { label: msg`Realized P/L`, pick: (d) => d.valuation?.realized_pnl_base },
  dividends: { label: msg`Dividends, all time`, pick: (d) => d.valuation?.dividends_base },
  interest: { label: msg`Interest, all time`, pick: (d) => d.valuation?.interest_base },
  fees: { label: msg`Fees, all time`, pick: (d) => d.valuation?.fees_base },
  taxes: { label: msg`Taxes, all time`, pick: (d) => d.valuation?.taxes_base },
  delta: { label: msg`Earned over the period`, periodic: true, pick: (d) => d.summary?.delta_base },
  change: {
    label: msg`Change in value`,
    periodic: true,
    pick: (d) => d.summary?.absolute_change_base,
  },
  flow: { label: msg`Paid in over the period`, periodic: true, pick: (d) => d.summary?.net_flow_base },
  netinvested: {
    label: msg`Invested capital`,
    periodic: true,
    pick: (d) => d.summary?.invested_capital_base,
  },
  avgcapital: {
    label: msg`Average capital`,
    periodic: true,
    pick: (d) => d.summary?.average_capital_base,
  },
  periodfees: { label: msg`Fees over the period`, periodic: true, pick: (d) => d.costs?.fees_base },
  periodtaxes: { label: msg`Taxes over the period`, periodic: true, pick: (d) => d.costs?.taxes_base },
  income: { label: msg`Income for the period`, periodic: true, pick: (d) => d.income?.net_base },
};

export interface RatioData {
  valuation?: PortfolioValuation;
  summary?: PeriodSummary;
  costs?: ChargeSummary;
  income?: IncomeSummary;
}

/** The two terms a ratio widget divides, defaulting to income against value — a yield. */
export function ratioTerms(cfg: Record<string, unknown>): { top: string; bottom: string } {
  const named = (key: string, fallback: string) =>
    typeof cfg[key] === "string" && cfg[key] in RATIO_TERMS ? (cfg[key] as string) : fallback;
  return { top: named("top", "income"), bottom: named("bottom", "value") };
}

/** The assumptions a FIRE tile was configured with; a blank one is not a zero. */
export function fireCfg(cfg: Record<string, unknown>) {
  const text = (key: string, fallback: string) => {
    const value = cfg[key];
    return typeof value === "string" && value.trim() !== "" ? value.trim() : fallback;
  };
  return {
    annual_spending: text("spending", ""),
    withdrawal_rate: text("withdrawal", "0.04"),
    expected_return: text("return", "0.05"),
    // Left blank, the pace is what the plans already contribute; a typed zero means none.
    contribution:
      typeof cfg.contribution === "string" && cfg.contribution.trim() !== "" ? cfg.contribution.trim() : null,
  };
}

/** How many benchmark lines one chart holds before it stops reading as a comparison. */
export const MAX_BENCHMARKS = 5;

/**
 * The instruments a benchmark chart compares against, in the order drawn — the order is also the
 * colour. `benchmark` is the single id boards stored before a chart could hold several.
 */
/** Whether a comparison chart also draws the cost of money. Off unless asked for. */
export function inflationOf(cfg: Record<string, unknown>): boolean {
  return cfg.inflation === true;
}

export function benchmarksOf(cfg: Record<string, unknown>): string[] {
  const own = Array.isArray(cfg.benchmarks)
    ? cfg.benchmarks
    : typeof cfg.benchmark === "string"
      ? [cfg.benchmark]
      : [];
  const ids = own.filter((id): id is string => typeof id === "string" && id !== "");
  return [...new Set(ids)].slice(0, MAX_BENCHMARKS);
}

export interface WidgetDef {
  label: MessageDescriptor;
  icon: Icon;
  description: MessageDescriptor;
  /** Catalog section: metrics, charts, lists, or text. */
  group: MessageDescriptor;
  /** Size a freshly added widget gets, in grid units (width in twelfths, height in rows). */
  size: { w: number; h: number };
  /** The smallest size the widget still reads at; dragging stops here. */
  min: { w: number; h: number };
  fields: Field[];
  defaults?: Record<string, unknown>;
  /** Borderless, titleless widget used as a section divider. */
  plain?: boolean;
  /** Title derived from widget settings. */
  titleOf?: (i18n: I18n, cfg: Record<string, unknown>) => string;
  /** Whether the period is worth showing: a metric tile consumes one only for some metrics. */
  periodicFor?: (cfg: Record<string, unknown>) => boolean;
  Render: (props: WidgetProps) => JSX.Element;
}

/** Resolve the widget period, falling back to the dashboard period. */
export function periodOf(widget: Widget, dashboard: PeriodId): PeriodId {
  const own = widget.cfg.period;
  return typeof own === "string" && own !== "" ? own : dashboard;
}

/** Small header text for period and breakdown when not in the title. */
export function widgetMeta(
  i18n: I18n,
  widget: Widget,
  def: WidgetDef,
  dashboard: PeriodId,
  ranges?: PeriodRange[],
): string {
  if (!def.fields.includes("period")) return "";
  if (def.periodicFor && !def.periodicFor(widget.cfg)) return "";
  // A stored id the axis no longer offers resolves to the same fallback the queries take,
  // so the header names the period that is actually on screen.
  const id = periodOf(widget, dashboard);
  return periodLabel(i18n, pickRange(ranges, id) ?? { id, name: null });
}

export function widgetTitle(i18n: I18n, widget: Widget, def: WidgetDef): string {
  const own = widget.cfg.title;
  if (typeof own === "string" && own.trim() !== "") return own;
  return def.titleOf ? def.titleOf(i18n, widget.cfg) : i18n._(def.label);
}

/** An empty taxonomy falls back to the always-available security breakdown. */
export function taxonomyOf(cfg: Record<string, unknown>): string | null {
  return typeof cfg.taxonomy === "string" && cfg.taxonomy ? cfg.taxonomy : null;
}

/** Same rate and window as the metric tiles, so one history pass serves both. */
export function useWidgetRisk(widget: Widget, date: DateString, period: PeriodId) {
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const query = useRisk(range, "0", "63", sourceOf(widget));
  return range ? query : null;
}

/** The target a widget is bound to, falling back to the first one the portfolio has: a fresh
 * widget shows something rather than an empty frame asking to be configured. */
export function useWidgetTarget(widget: Widget): { id: string | null; name: string | null } {
  const targets = useTargets();
  const chosen = typeof widget.cfg.target === "string" ? widget.cfg.target : "";
  const found = targets.data?.find((t) => t.id === chosen) ?? targets.data?.[0];
  return { id: found?.id ?? null, name: found?.name ?? null };
}

/** A source as one string, which is what a `<select>` can carry. Empty is "follow the picker". */
export function sourceKey(source: { kind: ScopeKind; id: string | null } | undefined): string {
  return source ? `${source.kind}:${source.id ?? ""}` : "";
}

export function sourceFromKey(value: string): DataScope | undefined {
  const at = value.indexOf(":");
  if (at < 0) return undefined;
  const kind = SCOPE_KINDS.find((k) => k === value.slice(0, at));
  return kind ? { kind, id: value.slice(at + 1) || null } : undefined;
}

/** The answer length a generated tile asked for, in output tokens; absent means no limit. */
export function lengthOf(cfg: Record<string, unknown>): number | null {
  const tokens = Number(cfg.max_tokens);
  return Number.isInteger(tokens) && tokens > 0 ? tokens : null;
}

/** How often a generated tile may rewrite itself, in hours; absent means only on request. */
export const REFRESH_HOURS: Record<string, number> = {
  daily: 24,
  weekly: 24 * 7,
  monthly: 24 * 30,
};

/** The chosen interval in milliseconds, or null for "only when asked" — which is the default. */
export function refreshEvery(cfg: Record<string, unknown>): number | null {
  const hours = typeof cfg.refresh === "string" ? REFRESH_HOURS[cfg.refresh] : undefined;
  return hours ? hours * 3600 * 1000 : null;
}
