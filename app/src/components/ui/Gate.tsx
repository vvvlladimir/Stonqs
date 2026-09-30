import type { ReactNode } from "react";

/** A screen before the app: one centred card and the ways out below it. */
export function Gate({
  title,
  lead,
  children,
  foot,
  wide,
  step,
  actions,
}: {
  title: ReactNode;
  lead?: ReactNode;
  children: ReactNode;
  foot?: ReactNode;
  /** A card holding a list rather than a few fields. */
  wide?: boolean;
  /** Where in a sequence of gates this one is: "Step 2 of 2". */
  step?: ReactNode;
  /** Buttons pinned to the card's foot, so a long card never scrolls them away. */
  actions?: ReactNode;
}) {
  return (
    <div className={wide ? "gate gate--wide" : "gate"}>
      <section className="box panel gate__card">
        <header className="gate__head">
          {step && <p className="gate__step">{step}</p>}
          <h1>{title}</h1>
          {lead && <p className="gate__lead">{lead}</p>}
        </header>
        {children}
        {actions && <div className="gate__actions">{actions}</div>}
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
