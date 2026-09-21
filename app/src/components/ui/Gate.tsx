import type { ReactNode } from "react";

/**
 * A screen standing before the app itself — choosing a profile, unlocking one, setting one up:
 * one centred card, and below it the ways out that are not the card's own question.
 */
export function Gate({
  title,
  lead,
  children,
  foot,
}: {
  title: ReactNode;
  lead?: ReactNode;
  children: ReactNode;
  foot?: ReactNode;
}) {
  return (
    <div className="gate">
      <section className="panel gate__card">
        <header className="gate__head">
          <h1>{title}</h1>
          {lead && <p className="gate__lead">{lead}</p>}
        </header>
        {children}
      </section>
      {foot && <div className="gate__foot">{foot}</div>}
    </div>
  );
}

/** A titled block under the card: other profiles, a developer shortcut. */
export function GateSection({ title, children }: { title: ReactNode; children: ReactNode }) {
  return (
    <section className="gate__section">
      <h2>{title}</h2>
      {children}
    </section>
  );
}
