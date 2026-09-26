import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { NativeMenuKind } from "../../../lib/api";
import type { ChoiceId, CommandId } from "../../../lib/commands";

/**
 * The macOS menu bar as data. An entry names a command, a choice, a native item or a list the
 * app arranges itself (the screens); `MenuBar` resolves it against the registry, so whether an
 * item is enabled, its accelerator and its label all come from where the command is defined.
 * Adding an item is a line here; adding a command is a row in `lib/commands/catalog.ts`.
 */
export type MenuSpec =
  | "separator"
  /** `keyless` for the second place a command is listed: one accelerator shown twice reads as two. */
  | { command: CommandId; label?: MessageDescriptor; keyless?: boolean }
  | { native: NativeMenuKind; label: MessageDescriptor }
  | { submenu: MessageDescriptor; items: MenuSpec[] }
  /** A submenu of the choice's options, checked at its current value; disabled when nobody publishes it. */
  | { choice: ChoiceId; label: MessageDescriptor }
  /** Overview and the favourites, with their ⌘1–9. */
  | { screens: "favorites" }
  /** Every section of the navigation as a submenu of its screens, in the user's order. */
  | { screens: "sections" };

export interface MenuSection {
  /** `null` is the application menu, which macOS titles with the app's name. */
  label: MessageDescriptor | null;
  items: MenuSpec[];
}

export const APP_NAME = "Stonqs";
const app = APP_NAME;

/* eslint-disable lingui/no-unlocalized-strings -- native item kinds; every label is `msg` */
export const MENU: MenuSection[] = [
  {
    label: null,
    items: [
      { command: "about", label: msg`About ${app}` },
      "separator",
      { command: "checkUpdates", label: msg`Check for Updates…` },
      "separator",
      { command: "settings", label: msg`Settings…` },
      "separator",
      { native: "Services", label: msg`Services` },
      "separator",
      { native: "Hide", label: msg`Hide ${app}` },
      { native: "HideOthers", label: msg`Hide Others` },
      { native: "ShowAll", label: msg`Show All` },
      "separator",
      { native: "Quit", label: msg`Quit ${app}` },
    ],
  },
  {
    label: msg`File`,
    items: [
      {
        submenu: msg`New`,
        items: [
          { command: "newTransaction", label: msg`Transaction` },
          { command: "newAccount", label: msg`Account` },
          { command: "newInstrument", label: msg`Instrument` },
          { command: "newPlan", label: msg`Plan` },
          { command: "newAlert", label: msg`Alert` },
          { command: "newWatchlist", label: msg`Watchlist` },
        ],
      },
      "separator",
      { command: "importFile", label: msg`Import…` },
      { command: "exportTransactions", label: msg`Export Transactions…` },
      "separator",
      { choice: "profile", label: msg`Profile` },
      { command: "lock" },
      "separator",
      { native: "CloseWindow", label: msg`Close Window` },
    ],
  },
  {
    label: msg`Edit`,
    items: [
      { native: "Undo", label: msg`Undo` },
      { native: "Redo", label: msg`Redo` },
      "separator",
      { native: "Cut", label: msg`Cut` },
      { native: "Copy", label: msg`Copy` },
      { native: "Paste", label: msg`Paste` },
      { native: "SelectAll", label: msg`Select All` },
      "separator",
      { command: "search" },
    ],
  },
  {
    label: msg`View`,
    items: [
      { command: "palette" },
      { command: "ai", label: msg`AI assistant` },
      "separator",
      { choice: "period", label: msg`Period` },
      { choice: "scope", label: msg`Data source` },
      { command: "asOfToday" },
      "separator",
      { choice: "theme", label: msg`Colour scheme` },
      "separator",
      { native: "Fullscreen", label: msg`Enter Full Screen` },
    ],
  },
  {
    label: msg`Go`,
    items: [
      { screens: "favorites" },
      "separator",
      { screens: "sections" },
      "separator",
      { command: "settings", keyless: true },
    ],
  },
  {
    label: msg`Market data`,
    items: [{ command: "refresh" }, { command: "downloadHistory" }, "separator", { command: "stopRefresh" }],
  },
  {
    label: msg`Window`,
    items: [
      { native: "Minimize", label: msg`Minimize` },
      { native: "Maximize", label: msg`Zoom` },
      "separator",
      { native: "BringAllToFront", label: msg`Bring All to Front` },
    ],
  },
  {
    label: msg`Help`,
    items: [
      { command: "tour", label: msg`Guided Tour` },
      { command: "help" },
      { command: "ai", label: msg`Ask the Assistant`, keyless: true },
    ],
  },
];
/* eslint-enable lingui/no-unlocalized-strings */
