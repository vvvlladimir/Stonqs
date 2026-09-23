import { msg } from "@lingui/core/macro";
import type { MessageDescriptor } from "@lingui/core";
import {
  ArrowsLeftRightIcon,
  BankIcon,
  BellIcon,
  CalendarDotsIcon,
  CertificateIcon,
  ChartLineUpIcon,
  ChartPieSliceIcon,
  CoinsIcon,
  EyeIcon,
  FileArrowUpIcon,
  FileTextIcon,
  GearIcon,
  ListBulletsIcon,
  ReceiptIcon,
  ScalesIcon,
  SquaresFourIcon,
  WaveSineIcon,
  type Icon,
} from "@phosphor-icons/react";

import type { ScreenId } from "../../lib/nav";
import type { NavPrefs } from "../../lib/uiState";

export interface NavScreen {
  id: ScreenId;
  title: MessageDescriptor;
  icon: Icon;
}

export interface NavSection {
  id: string;
  label: MessageDescriptor;
  screens: ScreenId[];
}

/** Always the first favourite and never unpinned: the way back, not a screen of a section. */
export const HOME: ScreenId = "dashboard";
/** Sits at the foot beside the lenses — it is about the app, not about one screen. */
export const SETTINGS: ScreenId = "settings";
/** Overview plus this many favourites are the phone's tabs; `Menu` is the last one. */
export const TABS = 4;

export const SCREENS: Record<ScreenId, NavScreen> = {
  dashboard: { id: "dashboard", title: msg`Overview`, icon: SquaresFourIcon },
  positions: { id: "positions", title: msg`Positions`, icon: ListBulletsIcon },
  transactions: { id: "transactions", title: msg`Transactions`, icon: ReceiptIcon },
  accounts: { id: "accounts", title: msg`Accounts`, icon: BankIcon },
  securities: { id: "securities", title: msg`Instruments`, icon: CertificateIcon },
  watchlist: { id: "watchlist", title: msg`Watchlist`, icon: EyeIcon },
  alerts: { id: "alerts", title: msg`Alerts`, icon: BellIcon },
  performance: { id: "performance", title: msg`Performance`, icon: ChartLineUpIcon },
  trades: { id: "trades", title: msg`Trades`, icon: ArrowsLeftRightIcon },
  risk: { id: "risk", title: msg`Risk`, icon: WaveSineIcon },
  allocation: { id: "allocation", title: msg`Allocation`, icon: ChartPieSliceIcon },
  income: { id: "income", title: msg`Income`, icon: CoinsIcon },
  plans: { id: "plans", title: msg`Plans`, icon: CalendarDotsIcon },
  rebalance: { id: "rebalance", title: msg`Rebalance`, icon: ScalesIcon },
  import: { id: "import", title: msg`Import`, icon: FileArrowUpIcon },
  reports: { id: "reports", title: msg`Reports`, icon: FileTextIcon },
  settings: { id: "settings", title: msg`Settings`, icon: GearIcon },
};

/** The shipped arrangement; the user's own order is laid over it by `arrange`. */
const SECTIONS: NavSection[] = [
  {
    id: "portfolio",
    label: msg`Portfolio`,
    screens: ["positions", "transactions", "accounts", "securities", "watchlist", "alerts"],
  },
  {
    id: "analysis",
    label: msg`Analysis`,
    screens: ["performance", "trades", "risk", "allocation", "income"],
  },
  { id: "planning", label: msg`Planning`, screens: ["plans", "rebalance"] },
  { id: "data", label: msg`Data`, screens: ["import", "reports"] },
];

const SECTION_SCREENS = new Set<string>(SECTIONS.flatMap((s) => s.screens));

export interface Arrangement {
  favorites: ScreenId[];
  sections: NavSection[];
  open: string | null;
}

/** The stored order, with what it no longer names dropped and what it never named appended. */
export function arrange(prefs: NavPrefs): Arrangement {
  const sections = ordered(
    prefs.sections,
    SECTIONS.map((s) => s.id),
  ).map((id) => {
    const shipped = SECTIONS.find((s) => s.id === id)!;
    return { ...shipped, screens: ordered(prefs.screens[id] ?? [], shipped.screens) };
  });
  const favorites = unique(prefs.favorites).filter((id): id is ScreenId => SECTION_SCREENS.has(id));
  const open = sections.some((s) => s.id === prefs.open) ? prefs.open : null;
  return { favorites, sections, open };
}

/** `stored` kept where it names a known id, then every known id it left out. */
function ordered<T extends string>(stored: string[], known: readonly T[]): T[] {
  const kept = unique(stored).filter((id): id is T => (known as readonly string[]).includes(id));
  return [...kept, ...known.filter((id) => !kept.includes(id))];
}

function unique(list: string[]): string[] {
  return [...new Set(list)];
}

/** `id` taken out of `list` and put back before `before`, or at the end. */
export function moveBefore<T>(list: readonly T[], id: T, before: T | null): T[] {
  const out = list.filter((x) => x !== id);
  const at = before === null ? -1 : out.indexOf(before);
  out.splice(at < 0 ? out.length : at, 0, id);
  return out;
}

export function sectionOf(sections: NavSection[], screen: ScreenId): NavSection | undefined {
  return sections.find((s) => s.screens.includes(screen));
}
