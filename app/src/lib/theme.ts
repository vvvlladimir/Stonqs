/** Applies theme through one document attribute shared by all portals and SVG. */

import { useEffect } from "react";

/** A plugin theme is a variation over its base scheme (ADR-0070). */
export type ThemePreference = "system" | "light" | "dark" | `plugin:${string}`;

const QUERY = "(prefers-color-scheme: dark)";

/** The element the plugin stylesheet is loaded into; there is only ever one theme applied. */
const STYLE_ID = "plugin-theme";

function systemTheme(): "light" | "dark" {
  return window.matchMedia(QUERY).matches ? "dark" : "light";
}

// A wire prefix, not a word: it is stored, parsed and compared, and never read by anybody.
// eslint-disable-next-line lingui/no-unlocalized-strings
const PLUGIN = "plugin:";

/** The `<plugin id>/<theme id>` part of a preference, or `null` for a built-in one. */
export function pluginTheme(preference: ThemePreference): string | null {
  return preference.startsWith(PLUGIN) ? preference.slice(PLUGIN.length) : null;
}

/** The preference that picks an installed theme, from the key the host listed it under. */
export function themeOfPlugin(key: string): ThemePreference {
  return `${PLUGIN}${key}`;
}

/** Whether a stored string names an installed theme rather than a built-in scheme. */
export function isPluginTheme(value: string): boolean {
  return value.startsWith(PLUGIN);
}

export function applyTheme(preference: ThemePreference, base: "light" | "dark" = "dark"): void {
  const built = pluginTheme(preference) ? base : preference === "system" ? systemTheme() : preference;
  document.documentElement.setAttribute("data-theme", built);
}

/** Injected, not linked, so a theme file needs no origin. */
export function applyPluginCss(css: string | null): void {
  const existing = document.getElementById(STYLE_ID);
  if (css === null) {
    existing?.remove();
    return;
  }
  const style = existing ?? document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = css;
  if (!existing) document.head.append(style);
}

/** Watches OS theme changes and returns an unsubscribe function. */
export function watchSystemTheme(onChange: () => void): () => void {
  const mql = window.matchMedia(QUERY);
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}

/** The base scheme applies at once and the stylesheet joins it; a removed plugin leaves the base. */
export function useTheme(preference: ThemePreference, css?: string, base?: "light" | "dark"): void {
  useEffect(() => {
    applyTheme(preference, base);
    if (preference === "system") return watchSystemTheme(() => applyTheme("system"));
  }, [preference, base]);

  useEffect(() => {
    applyPluginCss(pluginTheme(preference) ? (css ?? null) : null);
  }, [preference, css]);
}
