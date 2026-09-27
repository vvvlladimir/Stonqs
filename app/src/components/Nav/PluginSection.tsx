import type { CSSProperties } from "react";
import { CaretRightIcon, PuzzlePieceIcon } from "@phosphor-icons/react";
import { Trans } from "@lingui/react/macro";
import type { InstalledScreen } from "../../lib/types";

/**
 * Plugin screens: one section after the shipped ones, in the plugin list's order. Nothing here
 * is dragged or pinned — the stored arrangement names only what the app defines (ADR-0084).
 */
export function PluginSection({
  screens,
  active,
  open,
  opening,
  onToggle,
  onPick,
}: {
  screens: InstalledScreen[];
  /** The key of the plugin screen on display, if one is. */
  active: string | null;
  open: boolean;
  opening: boolean;
  onToggle: () => void;
  onPick: (key: string) => void;
}) {
  if (screens.length === 0) return null;
  return (
    <>
      <button type="button" className="nav__row nav__head" aria-expanded={open} onClick={onToggle}>
        <span className="nav__lbl">
          <Trans>Plugins</Trans>
        </span>
        <CaretRightIcon className="nav__caret" />
      </button>
      <div
        className="nav__fold"
        data-open={open ? "" : undefined}
        data-opening={opening ? "" : undefined}
        ref={(el) => {
          if (el) el.inert = !open;
        }}
      >
        <div className="nav__clip">
          <div className="nav__sub">
            {screens.map((p, i) => {
              const on = active === p.key;
              return (
                <button
                  key={p.key}
                  type="button"
                  className="nav__row nav__row--sub"
                  aria-current={on ? "page" : undefined}
                  style={{ "--i": i } as CSSProperties}
                  onClick={() => onPick(p.key)}
                >
                  <PuzzlePieceIcon weight={on ? "fill" : "regular"} />
                  <span className="nav__lbl">{p.name}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </>
  );
}
