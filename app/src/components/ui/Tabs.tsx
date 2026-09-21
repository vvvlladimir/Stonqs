import type { ReactNode } from "react";

/**
 * A strip of categories above the one it shows. The strip scrolls sideways rather than
 * wrapping, so the same shape holds at 380px and on a desktop window.
 */

export interface TabItem<T extends string> {
  id: T;
  label: string;
  icon?: ReactNode;
}

interface TabsProps<T extends string> {
  items: TabItem<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
  /** A tab's own actions, opened by a right click or the menu key on it. */
  onMenu?: (value: T, tab: HTMLElement) => void;
  children: ReactNode;
}

export function Tabs<T extends string>({ items, value, onChange, label, onMenu, children }: TabsProps<T>) {
  return (
    <div className="tabs">
      <nav className="tabs__strip" aria-label={label}>
        {items.map((item) => (
          <button
            key={item.id}
            type="button"
            className="tabs__tab"
            aria-current={item.id === value ? "page" : undefined}
            aria-haspopup={onMenu ? "menu" : undefined}
            onClick={() => onChange(item.id)}
            onContextMenu={
              onMenu &&
              ((e) => {
                e.preventDefault();
                onMenu(item.id, e.currentTarget);
              })
            }
            onKeyDown={
              onMenu &&
              ((e) => {
                if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
                  e.preventDefault();
                  onMenu(item.id, e.currentTarget);
                }
              })
            }
          >
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
      <div className="tabs__pane">{children}</div>
    </div>
  );
}
