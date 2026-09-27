import { useLingui } from "@lingui/react/macro";

/** The mark beside Alerts while crossings wait to be looked at. */
export function AlertsDot() {
  const { t } = useLingui();
  return <i className="tab-dot" aria-label={t`new crossings to look at`} />;
}
