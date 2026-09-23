import { useEffect, type RefObject } from "react";

const FOCUSABLE =
  // eslint-disable-next-line lingui/no-unlocalized-strings -- a CSS selector
  'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

// eslint-disable-next-line lingui/no-unlocalized-strings -- a CSS selector
export const FIELDS = "input, select, textarea";

export function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => !el.closest("[inert]") && el.getClientRects().length > 0,
  );
}

/**
 * What a modal dialog owes the keyboard (WAI-ARIA dialog pattern): focus moves in when it opens —
 * to a field that asked for it, else the first field, else the dialog itself — `Tab` cycles
 * inside it, and focus returns to whatever opened it once it closes.
 */
export function useDialogFocus(ref: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const opener = document.activeElement as HTMLElement | null;

    // `autoFocus` has already run by now; a dialog that placed focus itself keeps it. Under a
    // finger the dialog itself takes focus: a field would pull up the on-screen keyboard unasked.
    if (!root.contains(document.activeElement)) {
      const touch = window.matchMedia("(pointer: coarse)").matches;
      const field = touch ? undefined : focusables(root).find((el) => el.matches(FIELDS));
      (field ?? root).focus({ preventScroll: true });
    }

    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const list = focusables(root);
      if (list.length === 0) {
        e.preventDefault();
        return;
      }
      const first = list[0];
      const last = list[list.length - 1];
      const at = document.activeElement;
      if (e.shiftKey && (at === first || at === root)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && at === last) {
        e.preventDefault();
        first.focus();
      }
    };
    root.addEventListener("keydown", onKey);
    return () => {
      root.removeEventListener("keydown", onKey);
      // Only when focus would otherwise be lost: a dialog that opened another keeps its hands off.
      const lost =
        !document.activeElement ||
        document.activeElement === document.body ||
        root.contains(document.activeElement);
      if (lost && opener?.isConnected) opener.focus({ preventScroll: true });
    };
  }, [ref]);
}
