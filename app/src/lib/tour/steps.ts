import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { ScreenId } from "../nav";

/**
 * One stop of the guided tour.
 *
 * A step points at a control and says what it is for. It **never names a figure, an instrument
 * or an amount**: the same steps run over the demo portfolio a new profile is filled with and
 * over somebody's real one, and a sentence about a number would be wrong in one of them
 * (ADR-0077).
 */
export interface TourStep {
  id: string;
  /** Opened before the step is shown. */
  screen: ScreenId;
  /**
   * `data-tour` of the element to point at. Absent, the card is centred and the step is about
   * the screen as a whole. A named anchor that never appears is skipped, not waited for: a
   * widget can be deleted from the board and a dock control can be off on a narrow window.
   */
  anchor?: string;
  title: MessageDescriptor;
  body: MessageDescriptor;
}

export const STEPS: TourStep[] = [
  {
    id: "welcome",
    screen: "dashboard",
    title: msg`A quick look around`,
    body: msg`Twelve stops, about a minute. Leave at any time with Escape — the tour writes nothing and changes no setting.`,
  },
  {
    id: "board",
    screen: "dashboard",
    title: msg`Overview`,
    body: msg`Tiles you arrange yourself: drag one by its header to move it, any edge to resize it. Each tile can read a different period and a different set of accounts.`,
  },
  {
    id: "nav",
    screen: "dashboard",
    anchor: "nav",
    title: msg`Getting around`,
    body: msg`Screens are grouped here, and the ones you pin sit at the top. Press ⌘K for the command palette if you would rather type where you are going.`,
  },
  {
    id: "scope",
    screen: "positions",
    anchor: "scope",
    title: msg`Data source`,
    body: msg`A lens over everything the app reports: one account, a group, or the whole portfolio. It changes which accounts the figures are read from — never what is stored, and never the list of operations.`,
  },
  {
    id: "asof",
    screen: "positions",
    anchor: "asof",
    title: msg`As of`,
    body: msg`Moves every reading screen to a past date, so you see the portfolio as it stood. Forms keep using today, so looking at the past cannot backdate what you type.`,
  },
  {
    id: "positions",
    screen: "positions",
    title: msg`Positions`,
    body: msg`What is held right now, one row per instrument. Which columns you see is your choice, and the purchase figures name the method they were counted by.`,
  },
  {
    id: "transactions",
    screen: "transactions",
    title: msg`Transactions`,
    body: msg`Every operation, and where they are typed in. A purchase has two sides: the instrument arrives on a securities account while the money leaves a cash one.`,
  },
  {
    id: "accounts",
    screen: "accounts",
    title: msg`Accounts`,
    body: msg`Cash accounts hold money; securities accounts hold instruments and settle through a cash account. Keeping them apart is what lets fees, dividends and interest land where they belong.`,
  },
  {
    id: "import",
    screen: "import",
    title: msg`Import`,
    body: msg`A file from a broker, read through a wizard. Nothing is written until the last step, and a file imported twice adds nothing the second time.`,
  },
  {
    id: "period",
    screen: "performance",
    anchor: "period",
    title: msg`Period`,
    body: msg`The window a figure is measured over. The strip is yours to edit: periods can be added and the shipped ones hidden.`,
  },
  {
    id: "performance",
    screen: "performance",
    title: msg`Performance`,
    body: msg`Return measured two ways, because they answer different questions: one ignores your deposits and withdrawals, the other is built around when they happened. Ask the assistant which one you want.`,
  },
  {
    id: "allocation",
    screen: "allocation",
    title: msg`Allocation`,
    body: msg`Trees you define — asset class, region, sector, or anything else — with each instrument split across them. Targets set here are what the rebalancing screen compares against.`,
  },
];
