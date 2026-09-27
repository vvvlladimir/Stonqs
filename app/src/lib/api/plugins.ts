/** Installed plugins and what they read and keep. */

import { convertFileSrc } from "@tauri-apps/api/core";
import type { PeriodRange, Plugin, PluginList, DateString, WidgetRead } from "../types";
import type { BridgeData } from "../pluginBridge";
import { call, type Source } from "./core";

export const pluginsApi = {
  pluginsList: () => call<PluginList>("plugins_list"),
  /** Installs the folder the user picked; a path, because a plugin is a folder, not a file. */
  pluginInstall: (path: string) => call<Plugin>("plugin_install", { path }),
  pluginRemove: (id: string) => call<void>("plugin_remove", { id }),
  pluginThemeCss: (plugin: string, theme: string) => call<string>("plugin_theme_css", { plugin, theme }),
  /** A classification set's CSV, previewed and committed by the commands every taxonomy file
   *  goes through — the set has no path into the portfolio of its own. */
  pluginTaxonomyCsv: (plugin: string, set: string) => call<number[]>("plugin_taxonomy_csv", { plugin, set }),
  /** Where a plugin's page is served: the host's own scheme, spelled the way this platform
   *  spells a custom one (ADR-0083). `key` is `<plugin id>/<widget or screen id>`. */
  pluginPageUrl: (kind: "widget" | "screen", key: string) =>
    convertFileSrc(`${kind}/${key}`, "stonqs-plugin"),
  /** What a plugin page declared it reads, built by the host in the plugin API's own names
   *  (ADR-0083) — the same projection an assistant tool of that package is handed. */
  pluginReads: (reads: WidgetRead[], date: DateString, range: PeriodRange | undefined, source?: Source) =>
    call<BridgeData>("plugin_reads", {
      reads,
      date,
      from: range?.from ?? null,
      to: range?.to ?? null,
      source: source ?? null,
    }),
  /** A plugin's one document in the open profile; null before it saved one (ADR-0084). */
  pluginStateGet: (plugin: string) => call<unknown>("plugin_state_get", { plugin }),
  pluginStateSave: (plugin: string, document: unknown) =>
    call<void>("plugin_state_save", { plugin, document }),
};
