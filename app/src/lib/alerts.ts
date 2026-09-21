import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { today } from "./api";
import { formatDay, formatMoney } from "./format";
import type {
  AlertInput,
  CrossingRow,
  SecurityAlert,
  SecurityEventInput,
  SecurityEventRow,
  SecurityRow,
} from "./types";

/** The currency a new level is written in: what the quotes arrive in, else the instrument's own. */
export function limitCurrency(security: SecurityRow | undefined): string {
  return security?.quote_currency ?? security?.currency ?? "";
}

/** A price trigger starting at the current close, which the user then moves to where it should fire. */
export function newAlert(security: SecurityRow): AlertInput {
  return {
    security_id: security.id,
    kind: "PRICE",
    direction: "UP",
    price: security.last_close ?? "",
    currency: limitCurrency(security),
    date: today(),
    note: "",
  };
}

export function alertToInput(alert: SecurityAlert): AlertInput {
  return {
    id: alert.id,
    security_id: alert.security_id,
    kind: alert.kind,
    direction: alert.direction,
    price: alert.price ?? "",
    currency: alert.currency,
    date: alert.date ?? today(),
    note: alert.note ?? "",
  };
}

export function newNote(securityId: string): SecurityEventInput {
  return { security_id: securityId, date: today(), note: "" };
}

export function noteToInput(event: SecurityEventRow): SecurityEventInput {
  return { id: event.id, security_id: event.security_id, date: event.date, note: event.note ?? "" };
}

/** The text of an OS notification for one crossing; the host sends codes, the sentence is ours. */
export function alertNotification(i18n: I18n, row: CrossingRow): { title: string; body: string } {
  const { symbol } = row;
  const day = formatDay(row.date);
  const note = row.note ? `\n${row.note}` : "";
  if (row.direction === "REACHED") {
    return {
      title: i18n._(msg`${symbol}: date reached`),
      body: i18n._(msg`${day} has arrived.`) + note,
    };
  }
  const currency = row.currency ?? "";
  const level = formatMoney(row.level ?? "0", currency);
  const price = formatMoney(row.price ?? "0", currency);
  return {
    title:
      row.direction === "UP"
        ? i18n._(msg`${symbol} rose above ${level}`)
        : i18n._(msg`${symbol} fell below ${level}`),
    body: i18n._(msg`Closed at ${price} on ${day}.`) + note,
  };
}
