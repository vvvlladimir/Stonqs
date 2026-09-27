import { msg } from "@lingui/core/macro";
import { i18n } from "@lingui/core";

/** The app's lines about itself, written here because they are sentences (ADR-0023). */

/** The footer a saved report leaves with. One line: it is read beside a number, not instead of it. */
const EXPORT_NOTE = msg`Recorded with Stonqs, a portfolio tracker — not investment advice. Figures are derived from data you entered and from free third-party sources, with no guarantee of accuracy.`;

export function exportNote(): string {
  return i18n._(EXPORT_NOTE);
}
