import type { ReactNode } from "react";

/** A screen before the app: one centred card and the ways out below it. */
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
      <section className="box panel gate__card">
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
