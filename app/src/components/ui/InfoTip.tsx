import type { ReactNode } from "react";
import { InfoIcon } from "@phosphor-icons/react";

/** An info mark whose one-sentence explanation is a `data-tip`, shown on hover or keyboard focus. */
export function InfoTip({ text }: { text: string }) {
  return (
    <span className="infotip" tabIndex={0} role="img" aria-label={text} data-tip={text}>
      <InfoIcon aria-hidden />
    </span>
  );
}

/** A section heading with its explanation kept behind an info mark rather than printed beside it. */
export function InfoHeading({ title, info }: { title: ReactNode; info?: string }) {
  if (!info) return <h2>{title}</h2>;
  return (
    <div className="infohead">
      <h2>{title}</h2>
      <InfoTip text={info} />
    </div>
  );
}
