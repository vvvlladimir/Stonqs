import { createPortal } from "react-dom";
import { useLingui } from "@lingui/react/macro";
import type { ScreenId } from "../../lib/nav";
import { SCREENS, type NavSection } from "./model";
import type { DragState } from "./useReorder";

/** What follows the pointer while a row or a section is carried. */
export function DragGhost({ drag, sections }: { drag: DragState; sections: NavSection[] }) {
  const { i18n } = useLingui();
  const ghost = SCREENS[drag.item.id as ScreenId];
  const section = sections.find((s) => s.id === drag.item.id);
  return createPortal(
    <div
      className="nav__ghost"
      data-remove={drag.drop?.kind === "unfav" ? "" : undefined}
      style={{ left: drag.x, top: drag.y, width: drag.width }}
    >
      {ghost ? (
        <>
          <ghost.icon />
          <span className="nav__lbl">{i18n._(ghost.title)}</span>
        </>
      ) : (
        section && <span className="nav__lbl">{i18n._(section.label)}</span>
      )}
    </div>,
    document.body,
  );
}
