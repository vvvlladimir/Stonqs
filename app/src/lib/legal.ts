import { msg } from "@lingui/core/macro";
import { i18n } from "@lingui/core";

/**
 * The lines the app says about itself: what it is not, and where its figures come from. They are
 * written here rather than in the host because they are sentences, and the language is known only
 * on this side (ADR-0023) — a file exported in Russian must not leave with an English note.
 */

/** The footer a saved report leaves with. One line: it is read beside a number, not instead of it. */
const EXPORT_NOTE = msg`Recorded with Stonqs, a portfolio tracker — not investment advice. Figures are derived from data you entered and from free third-party sources, with no guarantee of accuracy.`;

export function exportNote(): string {
  return i18n._(EXPORT_NOTE);
}
