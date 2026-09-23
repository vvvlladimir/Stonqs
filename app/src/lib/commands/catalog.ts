import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { ScreenId } from "../nav";

/** Heading a command is listed under in the shortcut list. */
export type CommandGroup = "general" | "navigation" | "create" | "data" | "screen";

export interface CommandDef {
  /** `mod+k`, a bare key (`n`, `?`) or a two-key sequence (`g p`); may be empty. */
  keys: string[];
  label: MessageDescriptor;
  group: CommandGroup;
  /** Kept out of the palette, which lists screens and choices by itself. */
  hidden?: boolean;
  /**
   * The screen that answers the command. Run anywhere else, the registry opens that screen and
   * leaves an intent the screen's `<Command>` consumes on arrival — `mod+n` from Overview.
   */
  screen?: ScreenId;
}

/**
 * Every command the app has, whoever answers it. The keyboard, the palette, the shortcut list,
 * tooltips and the macOS menu bar all read this one table; a new command is a row here plus a
 * `useCommand` / `<Command>` where it is answered.
 */
/* eslint-disable lingui/no-unlocalized-strings -- bindings and ids; the labels are the text */
export const COMMANDS = {
  palette: { keys: ["mod+k"], label: msg`Command palette`, group: "general" },
  help: { keys: ["?", "mod+/"], label: msg`Keyboard shortcuts`, group: "general" },
  search: { keys: ["mod+f", "/"], label: msg`Search on this screen`, group: "general" },
  ai: { keys: ["mod+j"], label: msg`Open or close the AI assistant`, group: "general" },
  lock: { keys: ["mod+shift+l"], label: msg`Lock profile`, group: "general" },
  checkUpdates: { keys: [], label: msg`Check for updates`, group: "general" },

  newTransaction: {
    keys: ["mod+n"],
    label: msg`New transaction`,
    group: "create",
    screen: "transactions",
  },
  newAccount: { keys: [], label: msg`New account`, group: "create", screen: "accounts" },
  newInstrument: { keys: [], label: msg`Add instrument`, group: "create", screen: "securities" },
  newPlan: { keys: [], label: msg`New plan`, group: "create", screen: "plans" },
  newAlert: { keys: [], label: msg`New alert`, group: "create", screen: "alerts" },
  newWatchlist: { keys: [], label: msg`New watchlist`, group: "create", screen: "watchlist" },
  new: { keys: ["n"], label: msg`Create on this screen`, group: "create" },

  importFile: { keys: ["mod+shift+i"], label: msg`Import a file`, group: "data", screen: "import" },
  exportTransactions: {
    keys: ["mod+shift+e"],
    label: msg`Export transactions`,
    group: "data",
    screen: "transactions",
  },
  refresh: { keys: ["mod+r"], label: msg`Refresh quotes and rates`, group: "data" },
  downloadHistory: { keys: [], label: msg`Download history`, group: "data" },
  stopRefresh: { keys: [], label: msg`Stop the refresh`, group: "data" },

  settings: { keys: ["mod+,"], label: msg`Settings`, group: "navigation", hidden: true },
  screen: { keys: [], label: msg`Open a screen`, group: "navigation", hidden: true },
  fav: {
    keys: ["mod+1", "mod+2", "mod+3", "mod+4", "mod+5", "mod+6", "mod+7", "mod+8", "mod+9"],
    label: msg`Overview and favourites, in order`,
    group: "navigation",
    hidden: true,
  },
  goDashboard: { keys: ["g o"], label: msg`Go to Overview`, group: "navigation", hidden: true },
  goPositions: { keys: ["g p"], label: msg`Go to Positions`, group: "navigation", hidden: true },
  goTransactions: { keys: ["g t"], label: msg`Go to Transactions`, group: "navigation", hidden: true },
  goAccounts: { keys: ["g a"], label: msg`Go to Accounts`, group: "navigation", hidden: true },
  goSecurities: { keys: ["g i"], label: msg`Go to Instruments`, group: "navigation", hidden: true },
  goWatchlist: { keys: ["g w"], label: msg`Go to Watchlist`, group: "navigation", hidden: true },

  asOfToday: { keys: [], label: msg`Back to today`, group: "screen" },
  periodPrev: { keys: ["["], label: msg`Previous period`, group: "screen" },
  periodNext: { keys: ["]"], label: msg`Next period`, group: "screen" },
} satisfies Record<string, CommandDef>;
/* eslint-enable lingui/no-unlocalized-strings */

export type CommandId = keyof typeof COMMANDS;

export const COMMAND_IDS = Object.keys(COMMANDS) as CommandId[];

export function commandDef(id: CommandId): CommandDef {
  return COMMANDS[id];
}

/**
 * A set of mutually exclusive options owned by whoever shows them — the period strip, the
 * data scope, the colour scheme, the open profile. Published with `useChoice`; the menu bar
 * draws each as a submenu of checkable items and the palette lists its options.
 */
export type ChoiceId = "period" | "scope" | "theme" | "profile";
