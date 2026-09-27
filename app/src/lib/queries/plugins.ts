/** Installed plugins and what their pages read and keep. */

import { useQuery } from "@tanstack/react-query";
import { api, type Source } from "../api";
import type { DateString, PeriodRange, WidgetRead } from "../types";
import { keys } from "./keys";

/** What the app is extended with, and which of it can be used. */
export function usePlugins() {
  return useQuery({ queryKey: keys.plugins(), queryFn: api.pluginsList });
}

/** Kept forever once read: a local file. */
export function usePluginTheme(theme: string | null) {
  const [plugin, id] = (theme ?? "").split("/");
  return useQuery({
    queryKey: keys.pluginTheme(theme ?? undefined),
    queryFn: () => api.pluginThemeCss(plugin, id),
    enabled: Boolean(plugin && id),
    staleTime: Infinity,
  });
}

/** Only the declared reads are built — that is the whole permission (ADR-0088). A period read waits for a period. */
export function usePluginReads(
  reads: readonly WidgetRead[],
  date: DateString,
  range: PeriodRange | undefined,
  source?: Source,
) {
  const periodic = reads.includes("performance") || reads.includes("transactions");
  return useQuery({
    queryKey: keys.pluginReads(reads, date, range?.from, range?.to, source),
    queryFn: () => api.pluginReads([...reads], date, range, source),
    enabled: !periodic || range !== undefined,
  });
}

/** A plugin's own document in the profile, for a page that declared `storage` (ADR-0084). */
export function usePluginState(plugin: string | null) {
  return useQuery({
    queryKey: keys.pluginState(plugin ?? undefined),
    queryFn: () => api.pluginStateGet(plugin!),
    enabled: plugin !== null,
  });
}
