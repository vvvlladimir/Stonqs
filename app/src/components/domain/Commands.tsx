import { useLayoutEffect, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { CertificateIcon, LightningIcon, SlidersHorizontalIcon } from "@phosphor-icons/react";

import { api } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { COMMANDS, keyParts, useChoice, useCommand, useRegistry, type CommandId } from "../../lib/commands";
import { useNav, type ScreenId } from "../../lib/nav";
import { openProfile, restart } from "../../lib/profiles";
import {
  affects,
  useInvalidate,
  usePlugins,
  useProfiles,
  useScope,
  useSecurities,
  useSettings,
} from "../../lib/queries";
import { themeOfPlugin, type ThemePreference } from "../../lib/theme";
import { useUiState } from "../../lib/uiState";
import { useUpdates } from "../../lib/updates";
import type { DataScope, ScopeOption } from "../../lib/types";
import { arrange, HOME, SCREENS } from "../Nav/model";
import { Palette, type PaletteItem } from "../ui";
import { MenuBar } from "./menuBar/MenuBar";
import { useRefreshStatus } from "./MarketRefresh";
import { scopeLabel } from "./scopeLabel";
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

/** The shell answers at the lowest priority, so a screen's own answer wins while it is mounted. */
const SHELL = { priority: 0 };

/** Scope kind and id together are what tells two options apart; the portfolio has no id. */
const scopeKey = (o: Pick<ScopeOption, "kind" | "id">) => `${o.kind}:${o.id ?? ""}`;

/**
 * The shell's side of the command layer: the commands and choices that mean the same on every
 * screen, the palette and the shortcut list, and the macOS menu bar. Renders only overlays.
 */
export function Commands({ aiOpen, onAi }: { aiOpen: boolean; onAi: () => void }) {
  const { t, i18n } = useLingui();
  const nav = useNav();
  const registry = useRegistry();
  const [palette, setPalette] = useState(false);
  const [help, setHelp] = useState(false);

  // Where a command with a `screen` goes when the current screen does not answer it.
  useLayoutEffect(() => {
    registry?.navigate({ screen: nav.screen, go: nav.go });
  });

  useShellCommands({ aiOpen, onAi, openPalette: () => setPalette(true), openHelp: () => setHelp(true) });
  useShellChoices();

  const { ui } = useUiState();
  const securities = useSecurities();
  const card = useSecurityCard();
  const favKeys = [HOME, ...arrange(ui.nav).favorites].slice(0, COMMANDS.fav.keys.length);

  const items = useMemo<PaletteItem[]>(() => {
    if (!palette || !registry) return [];
    const commands = t`Commands`;
    const screens = t`Screens`;
    const options = t`Options`;
    const instruments = t`Instruments`;
    const out: PaletteItem[] = registry.available().flatMap(({ id, label }) => {
      if (id === "palette") return [];
      const binding = (COMMANDS[id].keys as string[])[0];
      return [
        {
          id: `c:${id}`,
          label: label ?? i18n._(COMMANDS[id].label),
          group: commands,
          hint: binding ? keyParts(binding) : undefined,
          icon: <LightningIcon />,
          run: () => registry.run(id, undefined, "palette"),
        },
      ];
    });
    for (const screen of Object.values(SCREENS)) {
      const Icon = screen.icon;
      const fav = favKeys.indexOf(screen.id);
      out.push({
        id: `s:${screen.id}`,
        label: i18n._(screen.title),
        group: screens,
        hint: fav >= 0 ? keyParts(COMMANDS.fav.keys[fav]) : undefined,
        icon: <Icon />,
        run: () => nav.go(screen.id),
      });
    }
    for (const [id, choice] of registry.choices()) {
      for (const option of choice.options) {
        out.push({
          id: `o:${id}:${option.value}`,
          label: `${choice.label}: ${option.label}`,
          group: options,
          searchOnly: true,
          icon: <SlidersHorizontalIcon />,
          run: () => registry.pick(id, option.value),
        });
      }
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
  }, [palette, securities.data, i18n.locale]);

  return (
    <>
      <MenuBar />
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

/** Commands whose answer does not depend on the screen. */
function useShellCommands({
  aiOpen,
  onAi,
  openPalette,
  openHelp,
}: {
  aiOpen: boolean;
  onAi: () => void;
  openPalette: () => void;
  openHelp: () => void;
}) {
  const { go } = useNav();
  const { ui } = useUiState();
  const settings = useSettings();
  const profiles = useProfiles();
  const updates = useUpdates();
  const asOf = useAsOf();
  const { running } = useRefreshStatus();
  const favorites = arrange(ui.nav).favorites;
  const isProtected = profiles.data?.profiles.find((p) => p.id === profiles.data?.open)?.protected ?? false;
  const checking = updates?.stage === "checking" || updates?.stage === "installing";

  useCommand("palette", openPalette, SHELL);
  useCommand("help", openHelp, SHELL);
  useCommand(
    "search",
    () => {
      const field = document.querySelector<HTMLInputElement>("main [data-search]");
      field?.focus();
      field?.select();
    },
    SHELL,
  );
  useCommand("refresh", () => void api.marketRefresh("catch_up"), { ...SHELL, enabled: !running });
  useCommand("downloadHistory", () => void api.marketRefresh("full"), { ...SHELL, enabled: !running });
  useCommand("stopRefresh", () => void api.marketRefreshCancel(), { ...SHELL, enabled: running });
  useCommand("settings", () => go("settings"), SHELL);
  useCommand("ai", onAi, { ...SHELL, enabled: (settings.data?.ai_enabled ?? false) || aiOpen });
  useCommand("lock", () => void api.profileLock().then(restart), { ...SHELL, enabled: isProtected });
  useCommand("checkUpdates", () => updates?.check(), { ...SHELL, enabled: updates !== null && !checking });
  useCommand("asOfToday", asOf.reset, { ...SHELL, enabled: !asOf.isToday });
  useCommand("screen", (arg) => typeof arg === "string" && go(arg as ScreenId), SHELL);
  useCommand(
    "fav",
    (arg) => {
      const target = [HOME, ...favorites][typeof arg === "number" ? arg : 0];
      if (target) go(target);
    },
    SHELL,
  );
  for (const [id, screen] of GO) {
    // A fixed list, so the hooks are called in the same order on every render.
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useCommand(id, () => go(screen), SHELL);
  }
}

/** Choices that belong to the app rather than to a screen: scheme, profile, data source. */
function useShellChoices() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const { ui, save } = useUiState();
  const plugins = usePlugins();
  const profiles = useProfiles();
  const scope = useScope();

  const setScope = useMutation({
    mutationFn: (value: DataScope) => api.scopeSet(value),
    onSuccess: () => invalidate(...affects.scope),
  });

  useChoice(
    "theme",
    {
      label: t`Colour scheme`,
      options: [
        { value: "system", label: t`System` },
        { value: "light", label: t`Light` },
        { value: "dark", label: t`Dark` },
        ...(plugins.data?.themes ?? []).map((theme) => ({
          value: themeOfPlugin(theme.key),
          label: theme.name,
        })),
      ],
      value: ui.theme,
      pick: (theme) => save({ ...ui, theme: theme as ThemePreference }),
    },
    0,
  );

  const list = profiles.data;
  useChoice(
    "profile",
    list
      ? {
          label: t`Profile`,
          options: list.profiles.map((p) => ({ value: p.id, label: p.name })),
          value: list.open,
          pick: (id) => void openProfile(id, list.open),
        }
      : null,
    0,
  );

  const data = scope.data;
  useChoice(
    "scope",
    data
      ? {
          label: t`Data source`,
          options: data.options.map((o) => ({ value: scopeKey(o), label: scopeLabel(i18n, o) })),
          value: scopeKey(data.scope),
          pick: (key) => {
            const option = data.options.find((o) => scopeKey(o) === key);
            if (option) setScope.mutate({ kind: option.kind, id: option.id });
          },
        }
      : null,
    0,
  );
}
