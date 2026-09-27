/**
 * The bridge between a plugin widget's frame and the app (ADR-0083): what the frame is handed, in
 * what shape, and which messages it may send back. The only module that knows the protocol; the
 * frame's side of it is `app/src-tauri/src/plugins/widget_shim.js`.
 *
 * The data is a **projection**, not the wire: field names here are the plugin API's own and
 * change only with `api`, so a rename in `lib/types` never breaks somebody else's package. The
 * host builds it (`plugins::reads`, through `plugin_reads`) — the same projection an assistant
 * tool of the package is handed — and these interfaces type what arrives;
 * `the_projection_is_the_bridges` pins the two together.
 * Money stays a string, instruments are named by ticker and name, never by an internal id.
 */
import { useEffect, useState } from "react";
import type { DateString, MoneyString, PeriodRange, TransactionKind } from "./types";

export const BRIDGE_API = 1;

export interface BridgeValuation {
  date: DateString;
  base_currency: string;
  total_value: MoneyString;
  securities_value: MoneyString;
  cash: MoneyString;
  cost_basis: MoneyString;
  unrealized_result: MoneyString;
  realized_result: MoneyString;
  dividends: MoneyString;
  interest: MoneyString;
  fees: MoneyString;
  taxes: MoneyString;
}

export interface BridgePosition {
  symbol: string;
  name: string;
  /** The quote currency of `price`; every other amount is in the base currency. */
  currency: string;
  quantity: MoneyString;
  price: MoneyString;
  value: MoneyString;
  cost_basis: MoneyString;
  unrealized_result: MoneyString;
  /** Share of the scope's value; `"0.25"` is a quarter. */
  weight: MoneyString;
  /** The last trading day's change as a fraction, when there is one. */
  day_change: MoneyString | null;
}

export interface BridgePositions {
  date: DateString;
  base_currency: string;
  total_value: MoneyString;
  rows: BridgePosition[];
}

export interface BridgePerformance {
  from: DateString;
  to: DateString;
  base_currency: string;
  twr: MoneyString;
  twr_annualized: MoneyString | null;
  xirr: MoneyString | null;
  start_value: MoneyString;
  end_value: MoneyString;
  /** Paid in minus taken out; `+` is money brought in. */
  net_flow: MoneyString;
  /** The change with the flows taken out. */
  earned: MoneyString;
  series: { date: DateString; value: MoneyString; flow: MoneyString }[];
}

export interface BridgeTransaction {
  /** Opaque and stable for as long as the row exists: what a plugin may remember a choice by. */
  id: string;
  date: DateString;
  kind: TransactionKind;
  /** The account's name. */
  account: string;
  symbol: string | null;
  /** In `currency`; before charges, as the app stores it. */
  amount: MoneyString;
  currency: string;
  /** `amount` in the base currency at the date's rate. */
  amount_base: MoneyString;
  /** The signed cash leg in the base currency; purchases are negative. */
  net_base: MoneyString;
  note: string | null;
}

export interface BridgeTransactions {
  base_currency: string;
  rows: BridgeTransaction[];
}

export interface BridgeData {
  /** The period the reads were answered over, for a page that reads over one. */
  period?: { from: DateString; to: DateString };
  valuation?: BridgeValuation;
  positions?: BridgePositions;
  performance?: BridgePerformance;
  transactions?: BridgeTransactions;
  /** The plugin's own document, for a screen that declared `storage`; null before the first save. */
  state?: unknown;
}

export interface BridgeContext {
  api: number;
  date: DateString;
  /** The tile's period, for a widget that reads over one. */
  period: { from: DateString; to: DateString } | null;
  base_currency: string | null;
  locale: string;
  theme: { scheme: "light" | "dark"; tokens: Record<string, string> };
}

/** What the frame may say. Anything else is ignored. */
export type FrameMessage =
  | { stonqs: 1; type: "ready" }
  | { stonqs: 1; type: "error"; code: "threw" | "no_render"; detail: string }
  | { stonqs: 1; type: "save"; state: unknown };

export function isFrameMessage(value: unknown): value is FrameMessage {
  if (!value || typeof value !== "object") return false;
  const m = value as Record<string, unknown>;
  if (m.stonqs !== 1) return false;
  if (m.type === "ready" || m.type === "save") return true;
  return m.type === "error" && (m.code === "threw" || m.code === "no_render") && typeof m.detail === "string";
}

export function renderMessage(context: BridgeContext, data: BridgeData) {
  return { stonqs: 1, type: "render", context, data };
}

/**
 * The theme properties a widget may paint with: colours, radii and type sizes — the look, not the
 * layout. Read off the app's root at render time, so a plugin theme reaches the frame too.
 */
const TOKENS = [
  "--bg",
  "--surface",
  "--surface-2",
  "--surface-3",
  "--border",
  "--border-strong",
  "--text",
  "--text-2",
  "--text-3",
  "--accent",
  "--accent-soft",
  "--pos",
  "--pos-soft",
  "--neg",
  "--neg-soft",
  "--warn",
  "--warn-soft",
  "--slot-1",
  "--slot-2",
  "--slot-3",
  "--slot-4",
  "--slot-5",
  "--slot-6",
  "--slot-7",
  "--slot-8",
  "--r",
  "--r-sm",
  "--r-input",
  "--fs-micro",
  "--fs-meta",
  "--fs-sm",
  "--fs-base",
  "--fs-lg",
  "--fs-xl",
];

export function themeTokens(): BridgeContext["theme"] {
  const root = document.documentElement;
  const style = getComputedStyle(root);
  const tokens: Record<string, string> = {};
  for (const name of TOKENS) {
    const value = style.getPropertyValue(name).trim();
    if (value) tokens[name] = value;
  }
  return { scheme: root.getAttribute("data-theme") === "light" ? "light" : "dark", tokens };
}

/** The period the frame is told about: only a periodic widget has one. */
export function periodOfRange(range: PeriodRange | undefined): BridgeContext["period"] {
  return range ? { from: range.from, to: range.to } : null;
}

/**
 * The theme as the frame should see it, re-read whenever the app's scheme or a plugin stylesheet
 * changes — both are a mutation of the document, which is cheaper to watch than to re-derive.
 */
export function useThemeTokens(): BridgeContext["theme"] {
  const [theme, setTheme] = useState(themeTokens);
  useEffect(() => {
    const observer = new MutationObserver(() => setTheme(themeTokens()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    observer.observe(document.head, { childList: true, subtree: true, characterData: true });
    return () => observer.disconnect();
  }, []);
  return theme;
}
