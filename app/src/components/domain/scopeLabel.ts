import { msg } from "@lingui/core/macro";
import type { I18n } from "@lingui/core";

import type { ScopeOption } from "../../lib/types";

/** The one place scope wording is composed, from the host's names and kind. */
export function scopeLabel(i18n: I18n, option: ScopeOption): string {
  switch (option.kind) {
    case "PORTFOLIO":
      return i18n._(msg`Whole portfolio · ${option.name}`);
    case "GROUP":
      return i18n._(msg`Group · ${option.name}`);
    case "ACCOUNT":
      return option.account_kind === "SECURITIES"
        ? i18n._(msg`Securities · ${option.name}`)
        : i18n._(msg`Cash · ${option.name}`);
    case "ACCOUNT_WITH_CASH":
      return i18n._(msg`Securities · ${option.name} + ${option.cash_name ?? ""}`);
  }
}
