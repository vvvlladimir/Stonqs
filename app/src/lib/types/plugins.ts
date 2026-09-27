/** What a plugin theme is a variation of: the built-in scheme its stylesheet does not restate. */
export type ThemeBase = "light" | "dark";

export interface PluginTheme {
  id: string;
  name: string;
  file: string;
  base: ThemeBase;
}

/** Why a plugin is, or is not, in use. A code from the host; the wording is written here. */
export type PluginStatus =
  | { status: "ok" }
  | { status: "api"; wants: number; speaks: number }
  | { status: "broken"; detail: string }
  /** The folder's name is not the manifest's id: moved or copied by hand, and offering nothing. */
  | { status: "misplaced"; manifest_id: string };

/** A broker layout a plugin brings, with the sample it proved itself against at install. */
export interface PluginLayout {
  id: string;
  file: string;
  sample: string;
}

/** A file reader a plugin brings: a WebAssembly component that turns bytes the app cannot read
 *  into its own transaction file. `extensions` is what it is offered; empty means anything. */
export interface PluginReader {
  id: string;
  file: string;
  sample: string;
  expected: string;
  extensions: string[];
}

/** A ready classification tree a plugin brings: the same CSV the taxonomy import reads. */
export interface PluginTaxonomy {
  id: string;
  name: string;
  file: string;
}

/** Operation wordings for a language the app does not speak, read after its own words and never
 *  instead of them. */
export interface PluginDictionary {
  id: string;
  file: string;
  sample: string;
}

/** A file writer a plugin brings: turns the app's own transaction file into another format. */
export interface PluginWriter {
  id: string;
  name: string;
  file: string;
  sample: string;
  expected: string;
  extension: string;
}

/** What a widget may be handed, from the closed list of this plugin API (ADR-0083). */
/** `transactions` is a screen's read only (ADR-0084). */
export type WidgetRead = "valuation" | "positions" | "performance" | "transactions";

/** A size on the board's grid: width in twelfths, height in rows. */
export interface GridSize {
  w: number;
  h: number;
}

/** A dashboard widget a plugin brings: one module, drawn in a frame with no origin. */
export interface PluginWidgetDef {
  id: string;
  name: string;
  description: string;
  file: string;
  reads: WidgetRead[];
  periodic: boolean;
  size: GridSize;
  min: GridSize;
}

/** A whole screen a plugin brings: a larger widget that follows the app's lenses. */
export interface PluginScreenDef {
  id: string;
  name: string;
  description: string;
  file: string;
  reads: WidgetRead[];
  periodic: boolean;
  /** Keeps one document in the profile. */
  storage: boolean;
}

/** An assistant tool a plugin brings: a WebAssembly component the assistant may call (ADR-0085). */
export interface PluginToolDef {
  id: string;
  name: string;
  description: string;
  file: string;
  schema: string;
  reads: WidgetRead[];
  sample: string;
  expected: string;
}

export type Plugin = {
  id: string;
  name: string;
  version: string;
  themes: PluginTheme[];
  layouts: PluginLayout[];
  readers: PluginReader[];
  taxonomies: PluginTaxonomy[];
  dictionaries: PluginDictionary[];
  writers: PluginWriter[];
  widgets: PluginWidgetDef[];
  screens: PluginScreenDef[];
  tools: PluginToolDef[];
} & PluginStatus;

/** One installed theme, addressed the way the stored preference addresses it. */
export interface InstalledTheme {
  /** `<plugin id>/<theme id>` — what the preference holds after `plugin:`. */
  key: string;
  name: string;
  plugin: string;
  base: ThemeBase;
}

/** One classification set on offer, addressed the way a command names it. */
export interface InstalledTaxonomySet {
  /** `<plugin id>/<set id>`. */
  key: string;
  name: string;
  plugin: string;
}

/** One export format on offer, addressed the way the save command names it. */
export interface InstalledWriter {
  /** `<plugin id>/<writer id>`. */
  key: string;
  name: string;
  plugin: string;
  /** The ending a saved file gets, without the dot. */
  extension: string;
}

export interface PluginList {
  plugins: Plugin[];
  /** Only the themes that can actually be applied. */
  themes: InstalledTheme[];
  /** Only the sets that can actually be created. */
  taxonomy_sets: InstalledTaxonomySet[];
  /** Only the export formats that can actually be written. */
  writers: InstalledWriter[];
  /** Only the widgets that can actually be placed. */
  widgets: InstalledWidget[];
  /** Only the screens that can actually be opened. */
  screens: InstalledScreen[];
  /** The assistant tools offered to the model. */
  tools: InstalledTool[];
  /** The plugin API this build speaks. */
  api: number;
}

/** One dashboard widget on offer, addressed the way a board stores its type after `plugin:`. */
export interface InstalledWidget {
  /** `<plugin id>/<widget id>`. */
  key: string;
  name: string;
  description: string;
  plugin: string;
  /** The plugin's own name, which every tile it draws carries. */
  plugin_name: string;
  reads: WidgetRead[];
  periodic: boolean;
  size: GridSize;
  min: GridSize;
}

/** One screen on offer, addressed the way the navigation hint names it. */
export interface InstalledScreen {
  /** `<plugin id>/<screen id>`. */
  key: string;
  name: string;
  description: string;
  plugin: string;
  plugin_name: string;
  reads: WidgetRead[];
  periodic: boolean;
  storage: boolean;
}

/** One assistant tool on offer, as the plugin list shows it. */
export interface InstalledTool {
  /** `<plugin id>/<tool id>`. */
  key: string;
  name: string;
  plugin: string;
  plugin_name: string;
  reads: WidgetRead[];
}
