import { useEffect, useMemo } from "react";
import type { I18n } from "@lingui/core";
import { useLingui } from "@lingui/react/macro";

import { setAppMenu, type AppMenuEntry, type AppMenuSection } from "../../../lib/api";
import {
  commandDef,
  COMMANDS,
  IS_MAC,
  menuAccelerator,
  useRegistry,
  useRegistryVersion,
  type ChoiceId,
  type CommandArg,
  type CommandId,
  type Registry,
} from "../../../lib/commands";
import { useUiState } from "../../../lib/uiState";
import { arrange, HOME, SCREENS, type Arrangement } from "../../Nav/model";
import { APP_NAME, MENU, type MenuSpec } from "./model";

/** The menu bar exists on a Mac with a pointer, not on an iPad reporting itself as one. */
const NATIVE_MENU = IS_MAC && typeof navigator !== "undefined" && navigator.maxTouchPoints === 0;
/** Many registrations land in one commit (a screen mounting); the bar is updated once after them. */
const SETTLE_MS = 40;

interface Context {
  i18n: I18n;
  registry: Registry;
  layout: Arrangement;
}

/**
 * An item's id carries what picking it does: `c` runs a command with its argument, `o` picks a
 * choice's option. The position keeps two listings of one command apart.
 */
function commandItem(
  ctx: Context,
  at: string,
  id: CommandId,
  text: string,
  arg?: CommandArg,
  keyless = false,
): AppMenuEntry {
  return {
    id: `c|${at}|${id}|${JSON.stringify(arg ?? null)}`,
    text,
    accelerator: keyless ? undefined : menuAccelerator(id, typeof arg === "number" ? arg : undefined),
    enabled: ctx.registry.can(id),
  };
}

function resolve(ctx: Context, spec: MenuSpec, at: string): AppMenuEntry[] {
  const { i18n, registry, layout } = ctx;
  if (spec === "separator") return ["separator"];
  if ("native" in spec) return [{ native: spec.native, text: i18n._(spec.label) }];
  if ("command" in spec) {
    const text = spec.label
      ? i18n._(spec.label)
      : (registry.label(spec.command) ?? i18n._(commandDef(spec.command).label));
    return [commandItem(ctx, at, spec.command, text, undefined, spec.keyless)];
  }
  if ("submenu" in spec) {
    const items = spec.items.flatMap((item, i) => resolve(ctx, item, `${at}.${i}`));
    return [{ id: `s|${at}`, submenu: i18n._(spec.submenu), items }];
  }
  if ("choice" in spec) {
    const choice = registry.choice(spec.choice);
    const options = choice?.options ?? [];
    return [
      {
        id: `s|${at}`,
        submenu: i18n._(spec.label),
        enabled: options.length > 0,
        items: options.map((option) => ({
          id: `o|${at}|${spec.choice}|${option.value}`,
          text: option.label,
          checked: option.value === choice?.value,
        })),
      },
    ];
  }
  if (spec.screens === "favorites") {
    const screens = [HOME, ...layout.favorites].slice(0, COMMANDS.fav.keys.length);
    return screens.map((screen, i) => commandItem(ctx, at, "fav", i18n._(SCREENS[screen].title), i));
  }
  return layout.sections.map((section) => ({
    id: `s|${at}|${section.id}`,
    submenu: i18n._(section.label),
    items: section.screens.map((screen) =>
      commandItem(ctx, at, "screen", i18n._(SCREENS[screen].title), screen),
    ),
  }));
}

/**
 * The macOS menu bar, drawn from `model.ts` and kept current with the registry: a command
 * nobody answers is greyed out, a choice is checked at its value. Renders nothing.
 */
export function MenuBar() {
  const { i18n } = useLingui();
  const registry = useRegistry();
  const version = useRegistryVersion();
  const { ui } = useUiState();
  const locale = i18n.locale;
  const navKey = JSON.stringify(ui.nav);

  const sections = useMemo<AppMenuSection[] | null>(() => {
    if (!NATIVE_MENU || !registry) return null;
    const ctx: Context = { i18n, registry, layout: arrange(ui.nav) };
    return MENU.map((section, s) => ({
      text: section.label ? i18n._(section.label) : APP_NAME,
      items: section.items.flatMap((spec, i) => resolve(ctx, spec, `${s}.${i}`)),
    }));
    // `version` is what says the registry changed; `navKey` stands for `ui.nav`, `locale` for `i18n`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registry, version, locale, navKey]);

  useEffect(() => {
    if (!sections || !registry) return;
    const timer = window.setTimeout(() => {
      setAppMenu(sections, (id) => pick(registry, id)).catch(() => {
        // Outside the Tauri host (a plain `vite` tab) there is no menu bar to set.
      });
    }, SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [sections, registry]);

  return null;
}

function pick(registry: Registry, id: string) {
  const [kind, , name, value] = id.split("|");
  if (kind === "c") registry.run(name as CommandId, JSON.parse(value) ?? undefined, "menu");
  else if (kind === "o") registry.pick(name as ChoiceId, id.split("|").slice(3).join("|"));
}
