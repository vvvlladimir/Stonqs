import type { ReactNode } from "react";
import { slotVar } from "../../lib/plot";

/** Presentational classification tree; traversal and calculations stay upstream. */
export function Tree({ children }: { children: ReactNode }) {
  return <div className="tree">{children}</div>;
}

interface NodeProps {
  name: ReactNode;
  /** Category palette slot, not an arbitrary color. */
  slot?: number;
  value?: ReactNode;
  weight?: ReactNode;
  active?: boolean;
  child?: boolean;
  /** Marks an item that has not been classified yet. */
  unassigned?: boolean;
  tools?: ReactNode;
  onClick?: () => void;
}

export function TreeNode({ name, slot, weight, active, child, unassigned, tools, onClick }: NodeProps) {
  const classes = [
    "tnode",
    active ? "tnode--active" : "",
    child ? "tnode--child" : "",
    unassigned ? "tnode--unassigned" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes} onClick={onClick} role={onClick ? "button" : undefined}>
      <span className="tnode__dot" style={slotVar(slot)} />
      <span className="tnode__name">{name}</span>
      {weight !== undefined && <span className="tnode__w num">{weight}</span>}
      {tools && <span className="tnode__tools">{tools}</span>}
    </div>
  );
}
