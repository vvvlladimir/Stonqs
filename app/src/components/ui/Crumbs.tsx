import { CaretRightIcon } from "@phosphor-icons/react";

/** Hierarchy path whose final item represents the current level. */
export interface Crumb {
  id: string;
  label: string;
}

export function Crumbs({ items, onPick }: { items: Crumb[]; onPick: (id: string) => void }) {
  return (
    <nav className="crumbs">
      {items.map((item, i) => {
        const last = i === items.length - 1;
        return (
          <span key={item.id} className="inline">
            {i > 0 && <CaretRightIcon className="sep" />}
            <button
              type="button"
              className={`crumb${last ? " crumb--now" : ""}`}
              disabled={last}
              onClick={() => onPick(item.id)}
            >
              {item.label}
            </button>
          </span>
        );
      })}
    </nav>
  );
}
