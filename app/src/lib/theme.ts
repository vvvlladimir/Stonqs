/** Applies theme through one document attribute shared by all portals and SVG. */

import { useEffect } from "react";

export type ThemePreference = "system" | "light" | "dark";

const QUERY = "(prefers-color-scheme: dark)";

function systemTheme(): "light" | "dark" {
  return window.matchMedia(QUERY).matches ? "dark" : "light";
}

export function applyTheme(preference: ThemePreference): void {
  const theme = preference === "system" ? systemTheme() : preference;
  document.documentElement.setAttribute("data-theme", theme);
}

/** Watches OS theme changes and returns an unsubscribe function. */
export function watchSystemTheme(onChange: () => void): () => void {
  const mql = window.matchMedia(QUERY);
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}

/** Keeps the document in step with the saved preference, and with the OS while it follows it. */
export function useTheme(preference: ThemePreference): void {
  useEffect(() => {
    applyTheme(preference);
    if (preference !== "system") return;
    return watchSystemTheme(() => applyTheme("system"));
  }, [preference]);
}
