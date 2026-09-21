import type { ReactNode } from "react";
import { InfoIcon, WarningIcon, XCircleIcon } from "@phosphor-icons/react";

/** Persistent notice for errors, warnings, or required user actions. */
export type BannerTone = "warn" | "info" | "bad";

const ICONS = { warn: WarningIcon, info: InfoIcon, bad: XCircleIcon };

export function Banner({
  tone = "warn",
  action,
  children,
}: {
  tone?: BannerTone;
  action?: ReactNode;
  children: ReactNode;
}) {
  const Icon = ICONS[tone];
  return (
    <div className={`banner${tone === "warn" ? "" : ` banner--${tone}`}`}>
      <Icon className="lead" weight="fill" />
      <span>{children}</span>
      {action && (
        <>
          <span className="spacer" />
          {action}
        </>
      )}
    </div>
  );
}
