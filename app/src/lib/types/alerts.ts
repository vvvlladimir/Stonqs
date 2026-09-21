/** Price alerts, the crossings they log, and the rules behind them. */

import type { DateString, MoneyString } from "./primitives";
/** A price level or a day on one instrument. */
export type AlertKind = "PRICE" | "DATE_REACHED";

/** Where a close stands against the level; a close at the level counts as above. */
export type AlertSide = "ABOVE" | "BELOW";

export type CrossingDirection = "UP" | "DOWN" | "REACHED";

/** Which crossings of its level a price trigger logs and announces. */
export type AlertDirection = "UP" | "DOWN" | "BOTH";

/** A trigger as stored. `side` and `checked_through` are the host's bookmark of the last check. */
export interface SecurityAlert {
  id: string;
  security_id: string;
  kind: AlertKind;
  /** The level of a price trigger, in `currency`. */
  price: MoneyString | null;
  currency: string | null;
  /** The day of a date rule. */
  date: DateString | null;
  note: string | null;
  created_on: DateString;
  side: AlertSide | null;
  checked_through: DateString | null;
  direction: AlertDirection;
}

/** Where the price stands against a trigger today. */
export interface AlertStatus {
  /** Latest close in the level's currency; null for a date rule. */
  price: MoneyString | null;
  price_date: DateString | null;
  side: AlertSide | null;
  /** `level / price − 1`: how far the price has to move, positive when the level is above. */
  distance: MoneyString | null;
}

/** Why a trigger's status could not be read; the sentence is the frontend's. */
export type AlertProblem = "missing_price" | "missing_rate";

/** One line of the log: a close that crossed a level, or a date rule's day arriving. */
export interface AlertCrossing {
  id: number;
  alert_id: string;
  date: DateString;
  direction: CrossingDirection;
  /** The level as it was when crossed. */
  level: MoneyString | null;
  price: MoneyString | null;
  currency: string | null;
  /** Looked at on the Alerts screen; an unseen crossing puts the dot on the navigation. */
  seen: boolean;
  notified: boolean;
}

export interface AlertRow {
  alert: SecurityAlert;
  symbol: string;
  name: string;
  status: AlertStatus | null;
  problem: AlertProblem | null;
  /** The latest crossings of this rule, newest first. */
  crossings: AlertCrossing[];
}

export interface CrossingRow extends AlertCrossing {
  kind: AlertKind;
  note: string | null;
  security_id: string;
  symbol: string;
  name: string;
}

export interface AlertInput {
  id?: string | null;
  security_id: string;
  kind: AlertKind;
  direction?: AlertDirection;
  price?: MoneyString | null;
  /** Absent means the currency the instrument's quotes arrive in. */
  currency?: string | null;
  date?: DateString | null;
  note?: string | null;
}

/** What the debug build's simulator does to one rule. */
export type DevAlertStep = "cross" | "reset";
