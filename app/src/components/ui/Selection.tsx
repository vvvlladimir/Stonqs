import { useState, type ReactNode } from "react";
import { Trans, useLingui } from "@lingui/react/macro";

/** Tracks bulk selection by stable IDs while exposing only visible items. */
export function useSelection(ids: string[]) {
  const [picked, setPicked] = useState<ReadonlySet<string>>(() => new Set());

  const visible = ids.filter((id) => picked.has(id));
  const shown = new Set(visible);

  const toggle = (id: string) =>
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  return {
    /** Selected IDs currently visible in the list. */
    ids: visible,
    count: visible.length,
    has: (id: string) => shown.has(id),
    toggle,
    /** Adds or removes a group without inverting individual selections. */
    setMany: (list: string[], on: boolean) =>
      setPicked((prev) => {
        const next = new Set(prev);
        for (const id of list) {
          if (on) next.add(id);
          else next.delete(id);
        }
        return next;
      }),
    /** Toggles all visible IDs, clearing them when all are selected. */
    toggleAll: () =>
      setPicked((prev) => {
        if (visible.length === ids.length && ids.length > 0) {
          const next = new Set(prev);
          for (const id of ids) next.delete(id);
          return next;
        }
        return new Set([...prev, ...ids]);
      }),
    clear: () => setPicked(new Set()),
    all: ids.length > 0 && visible.length === ids.length,
    some: visible.length > 0 && visible.length < ids.length,
  };
}

/** Row checkbox that does not bubble into row actions. */
export function Check({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
}) {
  return (
    <input
      type="checkbox"
      className="check"
      checked={checked}
      aria-label={label}
      onChange={onChange}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

/** Header checkbox with the DOM-only indeterminate state. */
export function CheckAll({
  all,
  some,
  onChange,
  label,
}: {
  all: boolean;
  some: boolean;
  onChange: () => void;
  label?: string;
}) {
  const { t } = useLingui();
  return (
    <input
      ref={(el) => {
        if (el) el.indeterminate = some;
      }}
      type="checkbox"
      className="check"
      checked={all}
      aria-label={label ?? t`Select all`}
      onChange={onChange}
    />
  );
}

/** Sticky bulk-action bar announced as a live status when the count changes. */
export function SelectionBar({
  count,
  onClear,
  children,
}: {
  count: number;
  onClear: () => void;
  children?: ReactNode;
}) {
  if (count === 0) return null;
  return (
    <div className="selbar" role="status">
      <span className="selbar__n num">
        <Trans>{count} selected</Trans>
      </span>
      <span className="spacer" />
      {children}
      <button type="button" className="iconbtn iconbtn--sm" onClick={onClear}>
        <Trans>Clear</Trans>
      </button>
    </div>
  );
}
