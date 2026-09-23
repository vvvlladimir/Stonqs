import { useId, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { MagnifyingGlassIcon } from "@phosphor-icons/react";
import { Plural } from "@lingui/react/macro";
import { useLayer } from "../../lib/shortcuts";
import { useDialogFocus } from "./focus";
import { Kbd } from "./Kbd";

export interface PaletteItem {
  id: string;
  label: string;
  /** Heading the item is listed under; groups keep the order of their first item. */
  group: string;
  /** Extra words it answers to: a ticker's ISIN, a screen's section. */
  keywords?: string;
  hint?: string[][];
  icon?: ReactNode;
  /** Listed only once something is typed — the instrument directory is not a menu. */
  searchOnly?: boolean;
  run: () => void;
}

/** How many search-only matches one group shows; the rest need a longer query. */
const SEARCH_LIMIT = 8;

function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Every typed word must occur; a label that starts with the query ranks above one containing it. */
function rank(items: PaletteItem[], query: string): PaletteItem[] {
  const q = fold(query.trim());
  if (!q) return items.filter((item) => !item.searchOnly);
  const words = q.split(/\s+/);
  const scored = items.flatMap((item) => {
    const label = fold(item.label);
    const text = `${label} ${fold(item.keywords ?? "")}`;
    if (!words.every((w) => text.includes(w))) return [];
    const score = label.startsWith(q)
      ? 0
      : text.split(/[\s./-]+/).some((w) => w.startsWith(words[0]))
        ? 1
        : 2;
    return [{ item, score }];
  });
  const groups = [...new Set(items.map((i) => i.group))];
  const out: PaletteItem[] = [];
  for (const group of groups) {
    const inGroup = scored.filter((s) => s.item.group === group).sort((a, b) => a.score - b.score);
    out.push(...inGroup.slice(0, inGroup[0]?.item.searchOnly ? SEARCH_LIMIT : undefined).map((s) => s.item));
  }
  return out;
}

/**
 * A command palette in the WAI-ARIA combobox pattern: focus stays in the field, the arrows move
 * the highlighted option (`aria-activedescendant`), `Enter` runs it and `Escape` closes.
 */
export function Palette({
  items,
  onClose,
  label,
  placeholder,
  empty,
}: {
  items: PaletteItem[];
  onClose: () => void;
  label: string;
  placeholder: string;
  empty: string;
}) {
  const box = useRef<HTMLDivElement>(null);
  const base = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  useLayer(onClose);
  useDialogFocus(box);

  const shown = useMemo(() => rank(items, query), [items, query]);
  const at = Math.min(active, Math.max(0, shown.length - 1));
  const optionId = (i: number) => `${base}-o${i}`;

  const move = (to: number) => {
    if (shown.length === 0) return;
    const next = (to + shown.length) % shown.length;
    setActive(next);
    document.getElementById(optionId(next))?.scrollIntoView({ block: "nearest" });
  };
  const pick = (item: PaletteItem | undefined) => {
    if (!item) return;
    onClose();
    item.run();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.nativeEvent.isComposing) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      move(at + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(at - 1);
    } else if (e.key === "PageDown") {
      e.preventDefault();
      move(Math.min(at + 8, shown.length - 1));
    } else if (e.key === "PageUp") {
      e.preventDefault();
      move(Math.max(at - 8, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      pick(shown[at]);
    }
  };

  const heads = shown.map((item, i) => (i === 0 || shown[i - 1].group !== item.group ? item.group : null));
  return createPortal(
    <div className="backdrop backdrop--top" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={box} className="palette" role="dialog" aria-modal="true" aria-label={label}>
        <label className="palette__field">
          <MagnifyingGlassIcon aria-hidden />
          <input
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls={`${base}-list`}
            aria-autocomplete="list"
            aria-activedescendant={shown.length > 0 ? optionId(at) : undefined}
            aria-label={label}
            placeholder={placeholder}
            autoFocus
            spellCheck={false}
            autoComplete="off"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKeyDown}
          />
        </label>
        <div id={`${base}-list`} className="palette__list" role="listbox" aria-label={label}>
          {shown.length === 0 && <div className="palette__empty">{empty}</div>}
          {shown.map((item, i) => {
            const head = heads[i];
            return [
              head && (
                <div key={`g:${head}`} className="palette__group" role="presentation">
                  {head}
                </div>
              ),
              <div
                key={item.id}
                id={optionId(i)}
                role="option"
                aria-selected={i === at}
                className="palette__item"
                onMouseMove={() => i !== at && setActive(i)}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pick(item)}
              >
                {item.icon && <span className="palette__icon">{item.icon}</span>}
                <span className="palette__label">{item.label}</span>
                {item.hint && <Kbd steps={item.hint} />}
              </div>,
            ];
          })}
        </div>
        <div className="sr-only" role="status">
          {query && <Plural value={shown.length} one="# result" other="# results" />}
        </div>
      </div>
    </div>,
    document.body,
  );
}
