import { useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { CaretRightIcon, ListIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import type { ScreenId } from "../../lib/nav";
import { useUiState, type NavPrefs } from "../../lib/uiState";
import { usePointerDrag } from "../../lib/pointerDrag";
import { ariaBinding, COMMANDS, useLayer } from "../../lib/commands";
import { AsOfPicker } from "../domain/AsOfPicker";
import { ScopePicker } from "../domain/ScopePicker";
import { arrange, HOME, moveBefore, SCREENS, SETTINGS, TABS, type NavSection } from "./model";
import { useReorder, type DragItem, type Drop } from "./useReorder";

interface Props {
  screen: ScreenId;
  go: (screen: ScreenId) => void;
  /** Alerts has crossings nobody looked at yet. */
  alertsDot: boolean;
}

/** How long the rows of a section that just opened keep their entrance animation. */
const OPENING_MS = 700;
/** A sheet pulled down further than this closes. */
const CLOSE_PULL = 80;

/**
 * The navigation: Favorites, sections folded to one open at a time, Settings and the two lenses
 * at the foot. On a wide window it is the side rail; below that the same element is a sheet
 * behind the tab bar's `Menu`, so the order and the pins are one thing at every width.
 * The arrangement is `UiState::nav`; see `model.ts` for how a stored order is read.
 */
export function Nav({ screen, go, alertsDot }: Props) {
  const { t, i18n } = useLingui();
  const { ui, save } = useUiState();
  const layout = arrange(ui.nav);
  const root = useRef<HTMLElement>(null);
  const [sheet, setSheet] = useState(false);
  const [opening, setOpening] = useState<string | null>(null);
  const [pull, setPull] = useState(0);

  const store = (patch: Partial<NavPrefs>) => save((ui) => ({ ...ui, nav: { ...ui.nav, ...patch } }));

  const onDrop = (item: DragItem, drop: Drop) => {
    const id = item.id as ScreenId;
    switch (drop.kind) {
      case "section":
        return store({
          sections: moveBefore(
            layout.sections.map((s) => s.id),
            item.id,
            drop.before,
          ),
        });
      case "screen": {
        const section = layout.sections.find((s) => s.id === item.section);
        if (!section) return;
        const screens = moveBefore(section.screens, id, drop.before as ScreenId | null);
        return store({ screens: { ...ui.nav.screens, [section.id]: screens } });
      }
      case "fav":
        return store({ favorites: moveBefore(layout.favorites, id, drop.before as ScreenId | null) });
      case "unfav":
        return store({ favorites: layout.favorites.filter((f) => f !== id) });
    }
  };
  const { drag, start } = useReorder(root, onDrop);

  const pullGesture = usePointerDrag({
    onMove: (_dx, dy) => setPull(Math.max(0, dy)),
    onEnd: (commit) => {
      if (commit && pull > CLOSE_PULL) setSheet(false);
      setPull(0);
    },
  });

  useLayer(() => setSheet(false), { active: sheet });

  const pick = (id: ScreenId) => {
    go(id);
    setSheet(false);
  };

  const toggle = (id: string) => {
    const next = layout.open === id ? null : id;
    if (next) {
      setOpening(next);
      window.setTimeout(() => setOpening((o) => (o === next ? null : o)), OPENING_MS);
    }
    store({ open: next });
  };

  const hasDot = (id: ScreenId) => id === "alerts" && alertsDot;
  const dot = <i className="tab-dot" aria-label={t`new crossings to look at`} />;
  const markAt = (key: string) => (drag?.mark?.key === key ? drag.mark.side : undefined);
  const carried = (kind: DragItem["kind"], id: string) =>
    drag?.item.kind === kind && drag.item.id === id ? "" : undefined;

  const row = (id: ScreenId, key: string, extra: string, item?: DragItem, index = 0) => {
    const s = SCREENS[id];
    const Icon = s.icon;
    const on = id === screen;
    const fav = key === "home" || key.startsWith("fav:") ? [HOME, ...layout.favorites].indexOf(id) : -1;
    const shortcut = COMMANDS.fav.keys[fav];
    return (
      <button
        key={key}
        type="button"
        className={`nav__row${extra}`}
        aria-current={on ? "page" : undefined}
        aria-keyshortcuts={shortcut ? ariaBinding(shortcut) : undefined}
        data-nav-fav={item?.kind === "fav" ? "" : undefined}
        data-nav-screen={item?.kind === "screen" ? "" : undefined}
        data-id={id}
        data-key={key}
        data-drop={markAt(key)}
        data-carried={item ? carried(item.kind, id) : undefined}
        style={{ "--i": index } as CSSProperties}
        onPointerDown={item ? start(item) : undefined}
        onClick={() => pick(id)}
      >
        <Icon weight={on ? "fill" : "regular"} />
        <span className="nav__lbl">{i18n._(s.title)}</span>
        {hasDot(id) && dot}
      </button>
    );
  };

  const section = (sec: NavSection) => {
    const open = layout.open === sec.id;
    const key = `section:${sec.id}`;
    return [
      <button
        key={key}
        type="button"
        className="nav__row nav__head"
        aria-expanded={open}
        data-nav-section=""
        data-id={sec.id}
        data-key={key}
        data-drop={markAt(key)}
        data-carried={carried("section", sec.id)}
        onPointerDown={start({ kind: "section", id: sec.id })}
        onClick={() => toggle(sec.id)}
      >
        <span className="nav__lbl">{i18n._(sec.label)}</span>
        {sec.screens.some(hasDot) && dot}
        <CaretRightIcon className="nav__caret" />
      </button>,
      <div
        key={`fold:${sec.id}`}
        className="nav__fold"
        data-open={open ? "" : undefined}
        data-opening={opening === sec.id ? "" : undefined}
        // A folded list is still in the DOM so folding can animate; it must not take focus.
        ref={(el) => {
          if (el) el.inert = !open;
        }}
      >
        <div className="nav__clip">
          <div className="nav__sub" data-nav-sub="">
            {sec.screens.map((id, i) =>
              row(id, `screen:${id}`, " nav__row--sub", { kind: "screen", id, section: sec.id }, i),
            )}
          </div>
        </div>
      </div>,
    ];
  };

  const tabs = [HOME, ...layout.favorites].slice(0, TABS);
  const ghost = drag && SCREENS[drag.item.id as ScreenId];
  const ghostSection = drag && layout.sections.find((s) => s.id === drag.item.id);

  return (
    <>
      <nav
        ref={root}
        className={`nav${sheet ? " nav--sheet" : ""}${pull ? " nav--pulled" : ""}`}
        data-tour="nav"
        style={pull ? ({ "--pull": `${pull}px` } as CSSProperties) : undefined}
        aria-label={t`Navigation`}
      >
        <div className="nav__grab" onPointerDown={pullGesture} aria-hidden="true" />
        <div className="nav__inner">
          <div className="nav__block nav__favs">
            <div className="nav__label">
              <Trans>Favorites</Trans>
            </div>
            <div className="nav__list" data-nav-favorites="" data-over={drag?.overFavorites ? "" : undefined}>
              {row(HOME, "home", "")}
              {layout.favorites.map((id) => row(id, `fav:${id}`, "", { kind: "fav", id }))}
            </div>
          </div>

          <div className="nav__block nav__sections">
            <div className="nav__label">
              <Trans>Sections</Trans>
            </div>
            {layout.sections.flatMap(section)}
            <div
              className="nav__end"
              data-nav-end=""
              data-key="sections-end"
              data-drop={markAt("sections-end")}
            />
          </div>

          <div className="nav__foot">
            {row(SETTINGS, "settings", "")}
            <div className="nav__lenses">
              <AsOfPicker />
              <ScopePicker />
            </div>
          </div>
        </div>
      </nav>

      <div className="nav__scrim" onClick={() => setSheet(false)} />

      <nav className="tabbar" aria-label={t`Tabs`}>
        {tabs.map((id) => {
          const s = SCREENS[id];
          const Icon = s.icon;
          const on = id === screen;
          return (
            <button
              key={id}
              type="button"
              className="tabbar__tab"
              aria-current={on ? "page" : undefined}
              onClick={() => pick(id)}
            >
              <Icon weight={on ? "fill" : "regular"} />
              <span className="nav__lbl">{i18n._(s.title)}</span>
              {hasDot(id) && dot}
            </button>
          );
        })}
        {/* A screen that is not a tab lights up Menu: you are somewhere inside it. */}
        <button
          type="button"
          className="tabbar__tab"
          data-on={tabs.includes(screen) ? undefined : ""}
          aria-expanded={sheet}
          onClick={() => setSheet((v) => !v)}
        >
          <ListIcon />
          <span className="nav__lbl">
            <Trans>Menu</Trans>
          </span>
          {alertsDot && !tabs.includes("alerts") && dot}
        </button>
      </nav>

      {drag &&
        createPortal(
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
              ghostSection && <span className="nav__lbl">{i18n._(ghostSection.label)}</span>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}
