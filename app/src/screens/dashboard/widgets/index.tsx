import { useMemo } from "react";
import { msg } from "@lingui/core/macro";
import { PuzzlePieceIcon } from "@phosphor-icons/react";
import { CHARTS } from "./catalog/charts";
import { LISTS } from "./catalog/lists";
import { NUMBERS } from "./catalog/numbers";
import { TEXT } from "./catalog/text";
import { PLUGIN_WIDGET, pluginWidgetKey } from "./model";
import { PluginWidget } from "./plugin";
import { usePlugins } from "../../../lib/queries";
import type { InstalledWidget } from "../../../lib/types";
import type { WidgetDef } from "./model";
import type { Widget } from "../../../lib/uiState";

export { periodOf, widgetMeta, widgetTitle } from "./model";
export type { Field, WidgetDef, WidgetProps } from "./model";

/** The dashboard catalog: one entry per widget the user can place. The picker groups by `group`,
 *  so the order here is only the order inside a group. */
export const WIDGETS: Record<string, WidgetDef> = {
  ...LISTS,
  ...NUMBERS,
  ...CHARTS,
  ...TEXT,
};

/** Create a catalog widget at the size the catalog says it reads best. */
export function makeWidget(type: string, id: string, def: WidgetDef): Widget {
  return { id, type, w: def.size.w, h: def.size.h, cfg: { ...(def.defaults ?? {}) } };
}

const PLUGINS_GROUP = msg`Plugins`;

/** A plugin's widget as the catalog describes one. Its name is the plugin's own words, so it is
 *  a descriptor with no translation behind it — the way a user period's name crosses. */
function pluginDef(info: InstalledWidget): WidgetDef {
  return {
    label: { id: info.name, message: info.name },
    icon: PuzzlePieceIcon,
    description: { id: info.description, message: info.description },
    group: PLUGINS_GROUP,
    size: info.size,
    min: info.min,
    fields: info.periodic ? ["title", "source", "period"] : ["title", "source"],
    Render: PluginWidget,
    plugin: { name: info.plugin_name, reads: info.reads },
  };
}

/** A tile whose plugin is gone: it stays on the board and says so (ADR-0083). */
const MISSING_PLUGIN: WidgetDef = {
  label: msg`Plugin widget`,
  icon: PuzzlePieceIcon,
  description: msg`A tile drawn by a plugin that is no longer installed.`,
  group: PLUGINS_GROUP,
  size: { w: 6, h: 6 },
  min: { w: 2, h: 2 },
  fields: ["title"],
  Render: PluginWidget,
};

/** Built-in widgets plus plugins' (`plugin:<plugin>/<widget>`). */
export function useWidgetCatalog() {
  // `widgets` is optional here only for IPC recorded before plugins could bring one.
  const widgets = usePlugins().data?.widgets;
  return useMemo(() => {
    const all: Record<string, WidgetDef> = { ...WIDGETS };
    for (const info of widgets ?? []) all[PLUGIN_WIDGET + info.key] = pluginDef(info);
    const of = (type: string): WidgetDef | undefined =>
      all[type] ?? (pluginWidgetKey(type) !== null ? MISSING_PLUGIN : undefined);
    return { all, of };
  }, [widgets]);
}
