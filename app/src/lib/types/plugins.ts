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
  { status: "ok" } | { status: "api"; wants: number; speaks: number } | { status: "broken"; detail: string };

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

export type Plugin = {
  id: string;
  name: string;
  version: string;
  themes: PluginTheme[];
  layouts: PluginLayout[];
  readers: PluginReader[];
} & PluginStatus;

/** One installed theme, addressed the way the stored preference addresses it. */
export interface InstalledTheme {
  /** `<plugin id>/<theme id>` — what the preference holds after `plugin:`. */
  key: string;
  name: string;
  plugin: string;
  base: ThemeBase;
}

export interface PluginList {
  plugins: Plugin[];
  /** Only the themes that can actually be applied. */
  themes: InstalledTheme[];
  /** The plugin API this build speaks. */
  api: number;
}
