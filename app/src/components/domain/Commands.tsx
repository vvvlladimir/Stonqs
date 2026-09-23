import { useEffect, useMemo, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { CertificateIcon, LightningIcon } from "@phosphor-icons/react";

import { api, setAppMenu, type AppMenuEntry, type AppMenuSection } from "../../lib/api";
import { useNav, type ScreenId } from "../../lib/nav";
import { restart } from "../../lib/profiles";
import { useProfiles, useSecurities, useSettings } from "../../lib/queries";
import {
  accelerator,
  COMMANDS,
  IS_MAC,
  keyParts,
  requestIntent,
  useCommand,
  useRunCommand,
  type CommandId,
} from "../../lib/shortcuts";
import { useUiState } from "../../lib/uiState";
import { arrange, HOME, SCREENS } from "../Nav/model";
import { Palette, type PaletteItem } from "../ui";
import { useSecurityCard } from "./SecurityCardProvider";
import { ShortcutHelp } from "./ShortcutHelp";

const GO: Array<[CommandId, ScreenId]> = [
  ["goDashboard", "dashboard"],
  ["goPositions", "positions"],
  ["goTransactions", "transactions"],
  ["goAccounts", "accounts"],
  ["goSecurities", "securities"],
  ["goWatchlist", "watchlist"],
];

/** The native menu bar exists on a Mac with a pointer, not on an iPad reporting itself as one. */
const NATIVE_MENU = IS_MAC && typeof navigator !== "undefined" && navigator.maxTouchPoints === 0;

/**
 * The shell's answers to the keyboard: the commands that mean the same on every screen, the
 * palette and the shortcut list, and — on macOS — the menu bar that carries the same commands.
 */
export function Commands({ aiOpen, onAi }: { aiOpen: boolean; onAi: () => void }) {
  const { t, i18n } = useLingui();
  const { go } = useNav();
  const { ui } = useUiState();
  const settings = useSettings();
  const profiles = useProfiles();
  const securities = useSecurities();
  const card = useSecurityCard();
  const registry = useRunCommand();
  const [palette, setPalette] = useState(false);
  const [help, setHelp] = useState(false);

  const favorites = arrange(ui.nav).favorites;
  const aiEnabled = settings.data?.ai_enabled ?? false;
  const isProtected = profiles.data?.profiles.find((p) => p.id === profiles.data?.open)?.protected ?? false;

  const shell = { priority: 0 };
  useCommand("palette", () => setPalette(true), shell);
  useCommand("help", () => setHelp(true), shell);
  useCommand(
    "search",
    () => {
      const field = document.querySelector<HTMLInputElement>("main [data-search]");
      field?.focus();
      field?.select();
    },
    shell,
  );
  useCommand(
    "newTransaction",
    () => {
      requestIntent("newTransaction");
      go("transactions");
    },
    shell,
  );
  useCommand("refresh", () => void api.marketRefresh("catch_up"), shell);
  useCommand("settings", () => go("settings"), shell);
  useCommand("ai", onAi, { ...shell, enabled: aiEnabled || aiOpen });
  useCommand("lock", () => void api.profileLock().then(restart), { ...shell, enabled: isProtected });
  useCommand(
    "fav",
    (index) => {
      const target = [HOME, ...favorites][index];
      if (target) go(target);
    },
    shell,
  );
  for (const [id, screen] of GO) {
    // A fixed list, so the hooks are called in the same order on every render.
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useCommand(id, () => go(screen), shell);
  }

  const favKeys = [HOME, ...favorites].slice(0, COMMANDS.fav.keys.length);

  // The menu bar is rebuilt when a text or an accelerator in it would change.
  const locale = i18n.locale;
  const favKey = favKeys.join(",");
  useEffect(() => {
    if (!NATIVE_MENU) return;
    const item = (id: CommandId, text = i18n._(COMMANDS[id].label), index = 0): AppMenuEntry => ({
      id: `${id}#${index}`,
      text,
      accelerator: accelerator(COMMANDS[id].keys[index]),
    });
    const native = (kind: Extract<AppMenuEntry, { native: unknown }>["native"], text: string) => ({
      native: kind,
      text,
    });
    const app = "Stonqs";
    /* eslint-disable lingui/no-unlocalized-strings -- native item kinds; every label is `t` */
    const sections: AppMenuSection[] = [
      {
        text: app,
        items: [
          native("About", t`About ${app}`),
          "separator",
          item("settings", t`Settings…`),
          "separator",
          native("Services", t`Services`),
          "separator",
          native("Hide", t`Hide ${app}`),
          native("HideOthers", t`Hide Others`),
          native("ShowAll", t`Show All`),
          "separator",
          native("Quit", t`Quit ${app}`),
        ],
      },
      {
        text: t`File`,
        items: [
          item("newTransaction"),
          ...(isProtected ? ["separator" as const, item("lock")] : []),
          "separator",
          native("CloseWindow", t`Close Window`),
        ],
      },
      {
        text: t`Edit`,
        items: [
          native("Undo", t`Undo`),
          native("Redo", t`Redo`),
          "separator",
          native("Cut", t`Cut`),
          native("Copy", t`Copy`),
          native("Paste", t`Paste`),
          native("SelectAll", t`Select All`),
          "separator",
          item("search"),
        ],
      },
      {
        text: t`View`,
        items: [
          item("palette"),
          ...(aiEnabled ? [item("ai", t`AI assistant`)] : []),
          "separator",
          ...favKeys.map((screen, i) => item("fav", i18n._(SCREENS[screen].title), i)),
          "separator",
          item("refresh"),
        ],
      },
      {
        text: t`Window`,
        items: [
          native("Minimize", t`Minimize`),
          native("Maximize", t`Zoom`),
          "separator",
          native("BringAllToFront", t`Bring All to Front`),
        ],
      },
      { text: t`Help`, items: [item("help", i18n._(COMMANDS.help.label), 1)] },
    ];
    /* eslint-enable lingui/no-unlocalized-strings */
    setAppMenu(sections, (picked) => {
      const [id, index] = picked.split("#");
      registry.run(id as CommandId, Number(index), "menu");
    }).catch(() => {
      // Outside the Tauri host (a plain `vite` tab) there is no menu bar to set.
    });
    // `t` and `i18n` follow `locale`; the registry is stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale, favKey, aiEnabled, isProtected]);

  const items = useMemo<PaletteItem[]>(() => {
    if (!palette) return [];
    const commands = t`Commands`;
    const screens = t`Screens`;
    const instruments = t`Instruments`;
    const out: PaletteItem[] = registry.available().flatMap(({ id, label }) =>
      id === "palette"
        ? []
        : [
            {
              id: `c:${id}`,
              label: label ?? i18n._(COMMANDS[id].label),
              group: commands,
              hint: keyParts(COMMANDS[id].keys[0]),
              icon: <LightningIcon />,
              run: () => registry.run(id, 0, "palette"),
            },
          ],
    );
    for (const screen of Object.values(SCREENS)) {
      const Icon = screen.icon;
      const fav = favKeys.indexOf(screen.id);
      out.push({
        id: `s:${screen.id}`,
        label: i18n._(screen.title),
        group: screens,
        hint: fav >= 0 ? keyParts(COMMANDS.fav.keys[fav]) : undefined,
        icon: <Icon />,
        run: () => go(screen.id),
      });
    }
    for (const s of securities.data ?? []) {
      out.push({
        id: `i:${s.id}`,
        label: `${s.symbol} · ${s.name}`,
        keywords: [s.isin, s.wkn].filter(Boolean).join(" "),
        group: instruments,
        searchOnly: true,
        icon: <CertificateIcon />,
        run: () => card.open(s.id),
      });
    }
    return out;
    // Built when the palette opens; what it lists does not move under the cursor while it is open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [palette, securities.data, locale]);

  return (
    <>
      {palette && (
        <Palette
          items={items}
          onClose={() => setPalette(false)}
          label={t`Command palette`}
          placeholder={t`Type a command, a screen or an instrument`}
          empty={t`Nothing matches`}
        />
      )}
      {help && <ShortcutHelp onClose={() => setHelp(false)} />}
    </>
  );
}
