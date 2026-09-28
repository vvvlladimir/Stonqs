import { useRef, useState, type CSSProperties, type PointerEventHandler, type ReactNode } from "react";
import { CaretRightIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import type { ScreenId } from "../../lib/nav";
import { useUiState, type NavPrefs } from "../../lib/uiState";
import { usePlugins } from "../../lib/queries";
import { usePointerDrag } from "../../lib/pointerDrag";
import { ariaBinding, COMMANDS, useLayer } from "../../lib/commands";
import { AsOfPicker } from "../domain/AsOfPicker";
import { ScopePicker } from "../domain/ScopePicker";
import { applyDrop, arrange, HOME, PLUGIN_SECTION, SCREENS, SETTINGS, TABS, type NavSection } from "./model";
import { AlertsDot } from "./AlertsDot";
import { DragGhost } from "./DragGhost";
import { PluginSection } from "./PluginSection";
import { TabBar } from "./TabBar";
import { useReorder, type DragItem, type Drop } from "./useReorder";

interface Props {
  screen: ScreenId;
  /** The navigation hint: which plugin screen is open while `screen` is `plugin`. */
  focus: string | null;
  go: (screen: ScreenId, focus?: string) => void;
  /** Alerts has crossings nobody looked at yet. */
  alertsDot: boolean;
}

/** How long the rows of a section that just opened keep their entrance animation. */
const OPENING_MS = 700;
/** A sheet pulled down further than this closes. */
const CLOSE_PULL = 80;

/** Side rail from 1000px, a sheet behind the tab bar's `Menu` below; order is `UiState::nav`. */
export function Nav({ screen, focus, go, alertsDot }: Props) {
  const { t, i18n } = useLingui();
  const { ui, save } = useUiState();
  const pluginScreens = usePlugins().data?.screens ?? [];
  const layout = arrange(ui.nav);
  const root = useRef<HTMLElement>(null);
  const [sheet, setSheet] = useState(false);
  const [opening, setOpening] = useState<string | null>(null);
  const [pull, setPull] = useState(0);

  const store = (patch: Partial<NavPrefs>) => save((ui) => ({ ...ui, nav: { ...ui.nav, ...patch } }));

  const onDrop = (item: DragItem, drop: Drop) => {
    const patch = applyDrop(layout, ui.nav, item, drop);
    if (patch) store(patch);
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

  const pick = (id: ScreenId, hint?: string) => {
    go(id, hint);
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
        {hasDot(id) && <AlertsDot />}
      </button>
    );
  };

  const tabs = [HOME, ...layout.favorites].slice(0, TABS);

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
            {layout.sections.map((sec) => (
              <NavSectionItem
                key={sec.id}
                section={sec}
                open={layout.open === sec.id}
                opening={opening === sec.id}
                dot={sec.screens.some(hasDot)}
                drop={markAt(`section:${sec.id}`)}
                carried={carried("section", sec.id)}
                onPointerDown={start({ kind: "section", id: sec.id })}
                onToggle={() => toggle(sec.id)}
              >
                {sec.screens.map((id, i) =>
                  row(id, `screen:${id}`, " nav__row--sub", { kind: "screen", id, section: sec.id }, i),
                )}
              </NavSectionItem>
            ))}
            <PluginSection
              screens={pluginScreens}
              active={screen === "plugin" ? focus : null}
              open={layout.open === PLUGIN_SECTION}
              opening={opening === PLUGIN_SECTION}
              onToggle={() => toggle(PLUGIN_SECTION)}
              onPick={(key) => pick("plugin", key)}
            />
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

      <TabBar
        tabs={tabs}
        screen={screen}
        sheet={sheet}
        alertsDot={alertsDot}
        onPick={pick}
        onMenu={() => setSheet((v) => !v)}
      />

      {drag && <DragGhost drag={drag} sections={layout.sections} />}
    </>
  );
}

/** A section heading and its folding list of screens. */
function NavSectionItem({
  section,
  open,
  opening,
  dot,
  drop,
  carried,
  onPointerDown,
  onToggle,
  children,
}: {
  section: NavSection;
  open: boolean;
  opening: boolean;
  dot: boolean;
  drop: string | undefined;
  carried: string | undefined;
  onPointerDown: PointerEventHandler<HTMLElement>;
  onToggle: () => void;
  children: ReactNode;
}) {
  const { i18n } = useLingui();
  const key = `section:${section.id}`;
  return (
    <>
      <button
        type="button"
        className="nav__row nav__head"
        aria-expanded={open}
        data-nav-section=""
        data-id={section.id}
        data-key={key}
        data-drop={drop}
        data-carried={carried}
        onPointerDown={onPointerDown}
        onClick={onToggle}
      >
        <span className="nav__lbl">{i18n._(section.label)}</span>
        {dot && <AlertsDot />}
        <CaretRightIcon className="nav__caret" />
      </button>
      <div
        className="nav__fold"
        data-open={open ? "" : undefined}
        data-opening={opening ? "" : undefined}
        // A folded list is still in the DOM so folding can animate; it must not take focus.
        ref={(el) => {
          if (el) el.inert = !open;
        }}
      >
        <div className="nav__clip">
          <div className="nav__sub" data-nav-sub="">
            {children}
          </div>
        </div>
      </div>
    </>
  );
}
