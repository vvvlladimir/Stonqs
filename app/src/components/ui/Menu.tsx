import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { CheckIcon, DotsThreeVerticalIcon } from "@phosphor-icons/react";
import { useLingui } from "@lingui/react/macro";

/** Accessible context menu shared by row context actions and menu buttons. */

export interface MenuItem {
  label: string;
  onSelect: () => void;
  /** Marks a destructive action. */
  danger?: boolean;
  disabled?: boolean;
  /** Explains why a disabled item is unavailable. */
  title?: string;
  /** Marks the item as the current choice. A menu where any item says so reserves the room for
   * the tick on every item, so picking another one does not shift the list sideways. */
  checked?: boolean;
}

interface Open {
  /** Row key used to expose the expanded state on its trigger. */
  key: string;
  x: number;
  y: number;
  /** Align the menu's right edge when opened from a button. */
  fromEnd: boolean;
  items: MenuItem[];
  /** Element that receives focus when the menu closes. */
  origin: HTMLElement | null;
}

const PAD = 8;

function Menu({ open, onClose }: { open: Open; onClose: () => void }) {
  const box = useRef<HTMLDivElement>(null);
  const items = useRef<Array<HTMLButtonElement | null>>([]);
  // Keep the menu focusable while its position is measured.
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const wanted = open.fromEnd ? open.x - width : open.x;
    setAt({
      left: Math.max(PAD, Math.min(wanted, window.innerWidth - width - PAD)),
      // Flip upward when the menu would be clipped below the viewport.
      top: open.y + height + PAD > window.innerHeight ? Math.max(PAD, open.y - height) : open.y,
    });
  }, [open.x, open.y, open.fromEnd]);

  useEffect(() => {
    items.current[0]?.focus();
  }, []);

  // Close when scrolling or resizing would detach the menu from its row.
  useEffect(() => {
    const close = () => onClose();
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [onClose]);

  useEffect(() => {
    const outside = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) onClose();
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [onClose]);

  const focusAt = (index: number) => {
    const list = items.current.filter(Boolean) as HTMLButtonElement[];
    if (list.length === 0) return;
    const wrapped = (index + list.length) % list.length;
    list[wrapped].focus();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const list = items.current.filter(Boolean) as HTMLButtonElement[];
    const current = list.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "ArrowDown") {
      e.preventDefault();
      focusAt(current + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      focusAt(current - 1);
    } else if (e.key === "Home") {
      e.preventDefault();
      focusAt(0);
    } else if (e.key === "End") {
      e.preventDefault();
      focusAt(list.length - 1);
    } else if (e.key === "Escape" || e.key === "Tab") {
      // Leave the temporary menu through its original trigger.
      e.preventDefault();
      onClose();
    }
  };

  const ticks = open.items.some((item) => item.checked !== undefined);

  return createPortal(
    <div
      ref={box}
      className="menu"
      role="menu"
      tabIndex={-1}
      onKeyDown={onKeyDown}
      onContextMenu={(e) => e.preventDefault()}
      style={{ left: at?.left ?? open.x, top: at?.top ?? open.y, opacity: at ? 1 : 0 }}
    >
      {open.items.map((item, i) => (
        <button
          key={item.label}
          ref={(el) => {
            items.current[i] = el;
          }}
          type="button"
          role={item.checked === undefined ? "menuitem" : "menuitemradio"}
          tabIndex={-1}
          // Keep disabled items focusable so their explanation remains reachable.
          aria-disabled={item.disabled}
          title={item.title}
          aria-checked={item.checked}
          className={item.danger ? "menu__item menu__item--danger" : "menu__item"}
          onClick={() => {
            if (item.disabled) return;
            onClose();
            item.onSelect();
          }}
        >
          {ticks && <span className="menu__tick">{item.checked && <CheckIcon weight="bold" />}</span>}
          {item.label}
        </button>
      ))}
    </div>,
    document.body,
  );
}

/** Keeps one menu state for the whole list instead of one per row. */
export function useMenu() {
  const { t } = useLingui();
  const [open, setOpen] = useState<Open | null>(null);

  const close = () => {
    open?.origin?.focus();
    setOpen(null);
  };

  const show = (
    key: string,
    list: MenuItem[],
    x: number,
    y: number,
    fromEnd: boolean,
    origin: HTMLElement | null,
  ) => {
    if (list.length === 0) return;
    setOpen({ key, items: list, x, y, fromEnd, origin });
  };

  /** Row props for opening the menu at the context-menu position. */
  const row = (key: string, list: MenuItem[]) => ({
    onContextMenu: (e: React.MouseEvent<HTMLElement>) => {
      e.preventDefault();
      const origin = e.currentTarget.querySelector<HTMLElement>("[data-menu-button]");
      show(key, list, e.clientX, e.clientY, false, origin);
    },
  });

  /** Button props for opening the menu below its right edge. */
  const button = (key: string, list: MenuItem[]) => {
    const open_ = (el: HTMLElement) => {
      const r = el.getBoundingClientRect();
      show(key, list, r.right, r.bottom + 4, true, el);
    };
    return (
      <button
        type="button"
        data-menu-button=""
        className="iconbtn iconbtn--sm kebab"
        aria-haspopup="menu"
        aria-expanded={open?.key === key}
        aria-label={t`Actions`}
        disabled={list.length === 0}
        onClick={(e) => open_(e.currentTarget)}
        onKeyDown={(e) => {
          if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
            e.preventDefault();
            open_(e.currentTarget);
          }
        }}
      >
        <DotsThreeVerticalIcon weight="bold" />
      </button>
    );
  };

  /** Opens the menu from an existing tab, chip, or heading element. */
  const openFrom = (key: string, list: MenuItem[], el: HTMLElement) => {
    if (list.length === 0) return;
    const r = el.getBoundingClientRect();
    show(key, list, r.left, r.bottom + 4, false, el);
  };

  const node: ReactNode = open && <Menu open={open} onClose={close} />;
  return { row, button, openFrom, node };
}
