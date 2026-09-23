import type { ElementType, HTMLAttributes, ReactNode } from "react";

/**
 * One row shape for every list in the product: a leading marker, a title with
 * its subtitle, and a right-hand value. Absent slots are simply not rendered,
 * so the row needs no per-shape column variants — only looks it can wear:
 * a line in a list, a box in a grid, or something you can tap.
 */

export interface ListProps {
  /**
   * How the rows sit together. `cards` stacks boxes with a gap, `grid` lays them out in columns,
   * and `picks` is a set of choices: gapped, roomier, and every box the same height — the option
   * that takes longer to explain must not look like the bigger answer.
   */
  variant?: "lines" | "cards" | "grid" | "picks";
  /** `ul` for a real list, `div` where the rows are buttons. */
  as?: "ul" | "div";
  className?: string;
  children: ReactNode;
}

export function List({ variant = "lines", as: Tag = "div", className, children }: ListProps) {
  const classes = ["rows"];
  if (variant !== "lines") classes.push(`rows--${variant}`);
  if (className) classes.push(className);
  return <Tag className={classes.join(" ")}>{children}</Tag>;
}

export interface ListRowProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  /** Selection checkbox, ahead of everything else. */
  pick?: ReactNode;
  /** Logo, day-and-month, colour dot or icon. */
  lead?: ReactNode;
  title?: ReactNode;
  sub?: ReactNode;
  /** Right-hand primary value. */
  value?: ReactNode;
  /** Second line under the value. */
  meta?: ReactNode;
  /** Cluster to the right of the value: a picker, a link — anything that is not a row action. */
  end?: ReactNode;
  /** Row actions: a menu button, an icon button. Shown on hover, always on touch. */
  actions?: ReactNode;
  /** Actions stay visible without hover: a short list whose rows exist to be managed. */
  showActions?: boolean;
  /** Full-width line below the row; a card's totals. */
  foot?: ReactNode;
  /** Boxed row: a card rather than a line. */
  box?: boolean;
  /** Tappable row: renders a button with its own hover and radius. */
  onClick?: HTMLAttributes<HTMLElement>["onClick"];
  /** Selected look for a tappable row. */
  on?: boolean;
  /** Oversized value, for a card whose number is the point. */
  big?: boolean;
  /** Title and subtitle wrap instead of being cut; for descriptions. */
  wrap?: boolean;
  /** Slots align to the top rather than the middle. */
  top?: boolean;
  as?: ElementType;
  className?: string;
  children?: ReactNode;
}

export function ListRow({
  pick,
  lead,
  title,
  sub,
  value,
  meta,
  end,
  actions,
  showActions,
  foot,
  box,
  onClick,
  on,
  big,
  wrap,
  top,
  as,
  className,
  children,
  ...rest
}: ListRowProps) {
  const classes = ["row"];
  if (box) classes.push("row--box");
  if (onClick) classes.push("row--tap");
  if (on) classes.push("row--on");
  if (big) classes.push("row--big");
  if (wrap) classes.push("row--wrap");
  if (top) classes.push("row--top");
  if (foot !== undefined) classes.push("row--foot");
  if (className) classes.push(className);

  // A tappable row is a button — unless it also carries actions, which are buttons of their own:
  // a button inside a button is invalid markup, and its click reaches the row underneath. Such a
  // row keeps the keyboard behaviour by hand rather than giving it up.
  const nested = onClick !== undefined && actions !== undefined;
  const Tag = (as ?? (onClick && !nested ? "button" : "div")) as ElementType;
  const side = value !== undefined || meta !== undefined;
  const aside = side || end !== undefined || actions !== undefined;

  return (
    <Tag
      className={classes.join(" ")}
      onClick={onClick}
      type={Tag === "button" ? "button" : undefined}
      role={nested ? "button" : undefined}
      tabIndex={nested ? 0 : undefined}
      onKeyDown={
        nested
          ? (e: React.KeyboardEvent<HTMLElement>) => {
              if (e.key !== "Enter" && e.key !== " ") return;
              if (e.target !== e.currentTarget) return;
              e.preventDefault();
              onClick?.(e as unknown as React.MouseEvent<HTMLElement>);
            }
          : undefined
      }
      {...rest}
    >
      {pick}
      {lead !== undefined && <span className="row__lead">{lead}</span>}
      {(title !== undefined || sub !== undefined) && (
        <span className="row__main">
          {title !== undefined && <span className="row__name">{title}</span>}
          {sub !== undefined && <span className="row__sub">{sub}</span>}
        </span>
      )}
      {children}
      {aside && (
        <span className="row__side">
          {side && (
            <span className="row__vals">
              {value !== undefined && <span className="row__val">{value}</span>}
              {meta !== undefined && <span className="row__pct">{meta}</span>}
            </span>
          )}
          {end}
          {actions !== undefined && (
            // The row's own click must not fire when what was pressed is an action on it.
            <span className={showActions ? "acts acts--shown" : "acts"} onClick={(e) => e.stopPropagation()}>
              {actions}
            </span>
          )}
        </span>
      )}
      {foot !== undefined && <span className="row__foot">{foot}</span>}
    </Tag>
  );
}
