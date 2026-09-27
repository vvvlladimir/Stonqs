import { ListIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { ScreenId } from "../../lib/nav";
import { AlertsDot } from "./AlertsDot";
import { SCREENS } from "./model";

/** The narrow layout's tabs: Overview, the first favourites, and Menu for everything else. */
export function TabBar({
  tabs,
  screen,
  sheet,
  alertsDot,
  onPick,
  onMenu,
}: {
  tabs: ScreenId[];
  screen: ScreenId;
  sheet: boolean;
  alertsDot: boolean;
  onPick: (id: ScreenId) => void;
  onMenu: () => void;
}) {
  const { t, i18n } = useLingui();
  return (
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
            onClick={() => onPick(id)}
          >
            <Icon weight={on ? "fill" : "regular"} />
            <span className="nav__lbl">{i18n._(s.title)}</span>
            {id === "alerts" && alertsDot && <AlertsDot />}
          </button>
        );
      })}
      {/* A screen that is not a tab lights up Menu: you are somewhere inside it. */}
      <button
        type="button"
        className="tabbar__tab"
        data-on={tabs.includes(screen) ? undefined : ""}
        aria-expanded={sheet}
        onClick={onMenu}
      >
        <ListIcon />
        <span className="nav__lbl">
          <Trans>Menu</Trans>
        </span>
        {alertsDot && !tabs.includes("alerts") && <AlertsDot />}
      </button>
    </nav>
  );
}
