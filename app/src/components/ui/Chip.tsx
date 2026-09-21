import type { ReactNode } from "react";

/** Clickable list filter rendered as a button. */
export function Chips({ children }: { children: ReactNode }) {
  return <div className="chips">{children}</div>;
}

export function Chip({
  active,
  disabled,
  title,
  onClick,
  children,
}: {
  active?: boolean;
  /** A filter with nothing behind it, or a wizard step not reachable yet. */
  disabled?: boolean;
  title?: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className="chip"
      aria-pressed={active}
      disabled={disabled}
      title={title}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
