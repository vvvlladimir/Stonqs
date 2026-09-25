import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "./api";
import { keys, useSettings } from "./queries";
import { isPluginTheme, type ThemePreference } from "./theme";
import shipped from "./defaultDashboard.json";

/** Persistent UI layout state stored by the host as opaque JSON. */

export interface Widget {
  id: string;
  type: string;
  /** Width in twelfths of the board, whatever the screen it is shown on. */
  w: number;
  /** Height in grid rows. */
  h: number;
  /** First column in twelfths, set by dragging the left edge; absent = wherever the flow puts it. */
  x?: number;
  /** Empty rows kept above the tile, set by dragging the top edge; absent = none. */
  y?: number;
  cfg: Record<string, unknown>;
}

export interface Dashboard {
  id: string;
  name: string;
  widgets: Widget[];
}

/** Bumped when a stored layout stops being readable as-is; `parseUiState` migrates. */
export const UI_VERSION = 3;

export interface UiState {
  /** Layout format the blob was written by; absent means the pre-resize one. */
  version: number;
  dashboards: Dashboard[];
  active_dashboard: string;
  /** Position-table column keys in display order. */
  position_columns: string[];
  /** Watchlist-table column keys in display order. */
  watch_columns: string[];
  /** How each of those two tables is ordered; `null` is the order the screen hands over. */
  position_sort: TableSort | null;
  watch_sort: TableSort | null;
  /** Colour scheme: an explicit one, or whatever the OS reports. */
  theme: ThemePreference;
  /** Width of the AI panel in pixels, on a screen wide enough for it to be a side panel. */
  ai_panel_width: number;
  /** The last brief each summary tile generated, by widget id. */
  ai_briefs: Record<string, Brief>;
  /** What the updater already asked about; only the frontend acts on it. */
  updates: UpdatePrefs;
  /** How the navigation is arranged. */
  nav: NavPrefs;
  /** Keyboard preferences; `lib/commands` is the only reader. */
  shortcuts: ShortcutPrefs;
  /** Whether the guided tour has been offered and answered. */
  tour: TourPrefs;
}

/**
 * The tour is offered once. `done` is set whether it was taken or declined — an offer repeated
 * after "no" is the same as not having asked. Starting it again is a command, never automatic.
 */
export interface TourPrefs {
  done: boolean;
}

/**
 * A bare-key shortcut (`n`, `g p`, `?`) can be fired by a speech-input user saying a word, so it
 * must be possible to turn them off (WCAG 2.1.4). `mod` shortcuts are not affected.
 */
export interface ShortcutPrefs {
  single_keys: boolean;
}

/**
 * The navigation's arrangement, stored as bare ids: which screens exist is `components/Nav`'s
 * business, and it drops an id it does not know and appends one missing here in shipped order,
 * so a screen added by a later build lands at the end of its section rather than nowhere.
 */
export interface NavPrefs {
  /** Pinned screens after Overview, in display order; on a phone the first three are the tabs. */
  favorites: string[];
  /** Section ids in display order. */
  sections: string[];
  /** Screen ids of each section, in display order. */
  screens: Record<string, string[]>;
  /** The expanded section, or `null` when all are folded. */
  open: string | null;
}

/**
 * The update check is a thing the app does to the user rather than for them, so it remembers
 * two answers: the version they said no to, and the day it last asked. A version declined once
 * is never offered again; a newer one is.
 */
export interface UpdatePrefs {
  /** Off means the check never runs on its own; `Check for updates` still works. */
  auto: boolean;
  /** Version the user chose to skip, or `null`. */
  skip: string | null;
  /** Local date of the last automatic check, `YYYY-MM-DD`. */
  checked: string | null;
}

/** A column key and a direction: what a table's heading was last clicked into. */
export interface TableSort {
  key: string;
  dir: "asc" | "desc";
}

/**
 * A generated summary, kept because generating it cost the user money: it survives a restart,
 * it is never regenerated on its own, and it carries the window it describes so a tile whose
 * period was changed afterwards can say the text is about a different one.
 */
export interface Brief {
  text: string;
  /** When it was generated, ISO. Compared against `lib/freshness` to mark it out of date. */
  at: string;
  from: string;
  to: string;
}

/** What the AI panel may be dragged between. Narrower than this and a table in a reply wraps to
 * nothing; wider and there is no app left beside it. */
export const AI_PANEL_MIN = 320;
export const AI_PANEL_MAX = 900;

const SHIPPED = (shipped as unknown as BoardFile).dashboard;

/**
 * The dashboard every install starts on. It is data on purpose — authored by exporting a board
 * from the app and pasting the file back into `defaultDashboard.json`, never by editing code.
 */
export const DEFAULT_UI: UiState = {
  version: UI_VERSION,
  // The file is exactly what "Export" writes, so the board sits under `dashboard`; its ids are
  // kept rather than re-minted, so every install names the shipped board the same way.
  dashboards: [SHIPPED],
  active_dashboard: SHIPPED.id,
  // `unrealized` predates purchase figures naming their method; it resolves to whichever
  // method the portfolio is kept under, so the default column follows that setting.
  position_columns: ["quantity", "price", "value", "weight", "day", "unrealized"],
  watch_columns: ["spark", "price", "day", "period", "range", "level", "leveldist", "quantity"],
  position_sort: null,
  watch_sort: null,
  theme: "system",
  ai_panel_width: 420,
  ai_briefs: {},
  updates: { auto: true, skip: null, checked: null },
  // The three screens the phone's bottom bar carried before favourites existed.
  nav: {
    favorites: ["positions", "transactions", "allocation"],
    sections: [],
    screens: {},
    open: "portfolio",
  },
  shortcuts: { single_keys: true },
  tour: { done: false },
};

/** Parses versioned or plugin-provided UI JSON with safe defaults. */
export function parseUiState(raw: unknown): UiState {
  if (!raw || typeof raw !== "object") return DEFAULT_UI;
  const value = raw as Partial<UiState>;
  const dashboards = Array.isArray(value.dashboards) ? value.dashboards.filter(isDashboard) : [];
  return {
    version: UI_VERSION,
    dashboards:
      dashboards.length > 0
        ? dashboards.map((board) => migrateBoard(board, version(value.version)))
        : DEFAULT_UI.dashboards,
    active_dashboard:
      typeof value.active_dashboard === "string" ? value.active_dashboard : DEFAULT_UI.active_dashboard,
    position_columns: Array.isArray(value.position_columns)
      ? value.position_columns.filter((c): c is string => typeof c === "string")
      : DEFAULT_UI.position_columns,
    watch_columns: Array.isArray(value.watch_columns)
      ? value.watch_columns.filter((c): c is string => typeof c === "string")
      : DEFAULT_UI.watch_columns,
    position_sort: tableSort(value.position_sort),
    watch_sort: tableSort(value.watch_sort),
    theme: isTheme(value.theme) ? value.theme : DEFAULT_UI.theme,
    ai_panel_width: clampPanel(value.ai_panel_width),
    ai_briefs: briefs(value.ai_briefs),
    updates: updatePrefs(value.updates),
    nav: navPrefs(value.nav),
    shortcuts: {
      single_keys:
        typeof value.shortcuts?.single_keys === "boolean"
          ? value.shortcuts.single_keys
          : DEFAULT_UI.shortcuts.single_keys,
    },
    // A board stored before the tour existed belongs to somebody already using the app: they
    // are not a new user, so the offer is counted as answered.
    tour: { done: typeof value.tour?.done === "boolean" ? value.tour.done : true },
  };
}

function navPrefs(value: unknown): NavPrefs {
  const prefs = value as Partial<NavPrefs> | undefined;
  if (!prefs || typeof prefs !== "object") return DEFAULT_UI.nav;
  const ids = (list: unknown) =>
    Array.isArray(list) ? list.filter((id): id is string => typeof id === "string") : [];
  const screens: Record<string, string[]> = {};
  if (prefs.screens && typeof prefs.screens === "object")
    for (const [section, list] of Object.entries(prefs.screens)) screens[section] = ids(list);
  return {
    favorites: Array.isArray(prefs.favorites) ? ids(prefs.favorites) : DEFAULT_UI.nav.favorites,
    sections: ids(prefs.sections),
    screens,
    open: typeof prefs.open === "string" || prefs.open === null ? prefs.open : DEFAULT_UI.nav.open,
  };
}

/** A blob written before the updater existed, or by hand: anything unreadable is the default. */
function updatePrefs(value: unknown): UpdatePrefs {
  const prefs = value as Partial<UpdatePrefs> | undefined;
  if (!prefs || typeof prefs !== "object") return DEFAULT_UI.updates;
  return {
    auto: typeof prefs.auto === "boolean" ? prefs.auto : DEFAULT_UI.updates.auto,
    skip: typeof prefs.skip === "string" ? prefs.skip : null,
    checked: typeof prefs.checked === "string" ? prefs.checked : null,
  };
}

/** A width from a hand-edited or plugin-written blob still has to be a width. */
/** Written by an older build, by hand, or by a plugin: anything not shaped like a brief is left out. */
function briefs(value: unknown): Record<string, Brief> {
  if (typeof value !== "object" || value === null) return {};
  const kept: Record<string, Brief> = {};
  for (const [id, brief] of Object.entries(value as Record<string, unknown>)) {
    const b = brief as Partial<Brief>;
    if (typeof b?.text !== "string" || typeof b.at !== "string") continue;
    kept[id] = { text: b.text, at: b.at, from: String(b.from ?? ""), to: String(b.to ?? "") };
  }
  return kept;
}

/** A stored order names a column that may no longer exist; the table drops such a key itself. */
function tableSort(value: unknown): TableSort | null {
  const sort = value as Partial<TableSort> | null;
  if (!sort || typeof sort.key !== "string") return null;
  return { key: sort.key, dir: sort.dir === "asc" ? "asc" : "desc" };
}

function clampPanel(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return DEFAULT_UI.ai_panel_width;
  return Math.min(Math.max(Math.round(value), AI_PANEL_MIN), AI_PANEL_MAX);
}

function isTheme(value: unknown): value is ThemePreference {
  if (typeof value !== "string") return false;
  // A plugin theme is stored by name, and the plugin behind it may be gone by the time this is
  // read — `useTheme` falls back to its base scheme rather than the preference being dropped,
  // because uninstalling a theme for one session should not forget it was chosen.
  return value === "system" || value === "light" || value === "dark" || isPluginTheme(value);
}

/**
 * A board written before widgets had a height. The old `span` counted quarters of the board,
 * so it triples into twelfths; the height it never had comes from the catalog, which is also
 * where an unknown type is caught.
 */
function migrateBoard(board: Dashboard, from: number): Dashboard {
  // A board written while `newId` was a bare timestamp holds one id for every widget minted in
  // the same millisecond, and an id is what tells two tiles apart: resizing one then resized
  // them all. Repeats are re-minted on read, so such a board heals on the next load.
  const seen = new Set<string>();
  const widgets = board.widgets
    .map((w) => migrateWidget(w, from))
    .filter((w): w is Widget => w !== null)
    .map((w) => {
      const id = seen.has(w.id) ? newId("w") : w.id;
      seen.add(id);
      return id === w.id ? w : { ...w, id };
    });
  return { ...board, widgets };
}

function migrateWidget(raw: unknown, from: number): Widget | null {
  const widget = raw as Widget & { span?: number };
  if (!widget || typeof widget.id !== "string" || typeof widget.type !== "string") return null;
  const fallback = FALLBACK_SIZE[widget.type] ?? { w: 6, h: 6 };
  const stored = size(widget.h);
  const track = MERGED_TRACKS[widget.type];
  const cfg = widget.cfg && typeof widget.cfg === "object" ? widget.cfg : {};
  return {
    id: widget.id,
    type: track ? "progress" : widget.type,
    w: size(widget.w) ?? (size(widget.span) ? size(widget.span)! * 3 : fallback.w),
    // Version 3 cut the row to a third of its height, so a height written before it counts
    // three of today's rows. A widget that never had one takes the fallback, already in rows.
    h: stored === null ? fallback.h : from >= 3 ? stored : stored * 3,
    ...offset("x", widget.x),
    ...offset("y", widget.y),
    cfg: track ? { ...cfg, track } : cfg,
  };
}

/**
 * The three tiles that became one. A goal, a contribution limit and financial independence are
 * one shape — a figure over a track — so the subject moved into the widget's own settings; the
 * old type is what names it. Type-based rather than version-gated, so a board file exported by
 * an older build reads the same way a stored one does.
 */
const MERGED_TRACKS: Record<string, string> = { goal: "goal", limit: "limit", fire: "fire" };

/** The format a blob was written by; anything unmarked predates the versioning. */
function version(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 1;
}

/**
 * Sizes for a widget whose type the catalog no longer describes, or never did: the layout
 * module must stay readable without importing the dashboard screen, so this is the one place
 * the two duplicate each other, and it is only a fallback.
 */
const FALLBACK_SIZE: Record<string, { w: number; h: number }> = {
  metric: { w: 3, h: 3 },
  heading: { w: 12, h: 1 },
};

/** A pinned column or a strip of empty rows, kept only when it is one; zero is a real column. */
function offset(key: "x" | "y", value: unknown): Partial<Pick<Widget, "x" | "y">> {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) return {};
  const n = Math.round(value);
  return key === "y" && n === 0 ? {} : { [key]: n };
}

function size(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.round(value) : null;
}

function isDashboard(value: unknown): value is Dashboard {
  const board = value as Dashboard;
  return (
    Boolean(board) &&
    typeof board.id === "string" &&
    typeof board.name === "string" &&
    Array.isArray(board.widgets)
  );
}

/**
 * A write to the UI state, expressed as a change to whatever is stored *now*.
 *
 * The blob is one document with a dozen writers in it — the tour, the updater, the board, two
 * column pickers — and it is saved whole. A writer that builds its next state out of the copy
 * it rendered with therefore puts back every field another writer changed in the meantime,
 * which is how declining the tour was undone by the daily update check landing a moment later.
 * The patch runs against the freshest copy instead, so two writers touching different fields
 * cannot overwrite each other.
 */
export type UiPatch = (current: UiState) => UiState;

export function useUiState() {
  const queryClient = useQueryClient();
  const settings = useSettings();

  const save = useMutation({
    mutationFn: (next: UiState) => api.uiStateSave(next),
    // Update optimistically so layout changes appear without a visible jump.
    onMutate: async (next: UiState) => {
      await queryClient.cancelQueries({ queryKey: keys.settings() });
      const previous = queryClient.getQueryData(keys.settings());
      queryClient.setQueryData(keys.settings(), (old: unknown) =>
        old && typeof old === "object" ? { ...old, ui: next } : old,
      );
      return { previous };
    },
    onError: (_error, _next, context) => {
      if (context?.previous) queryClient.setQueryData(keys.settings(), context.previous);
    },
  });

  // The cache rather than this render's `settings.data`: an optimistic write by another writer
  // one tick ago is already there, and that is the copy a patch has to be applied to. Nothing
  // in it yet means the settings have not arrived, and a patch over the defaults would store
  // them over whatever is really on disk, so the write is dropped instead.
  const stored = () => queryClient.getQueryData(keys.settings()) as { ui?: unknown } | undefined;

  return {
    ui: parseUiState(settings.data?.ui),
    ready: settings.isSuccess,
    /** Exposes load errors instead of silently replacing persisted settings. */
    loadError: settings.error,
    save: (next: UiPatch) => {
      const current = stored();
      if (current) save.mutate(next(parseUiState(current.ui)));
    },
    error: save.error,
  };
}

/**
 * An ID unique within one JSON document — distinct per call, not per millisecond: a board is
 * duplicated and imported by mapping over its widgets in one go, and a bare timestamp hands
 * every one of them the same id, which is what tells two tiles apart.
 */
let minted = 0;
export function newId(prefix: string): string {
  minted += 1;
  return `${prefix}-${Date.now().toString(36)}-${minted.toString(36)}`;
}

/**
 * A board as a file. Export and import travel through the same shape the host stores, so a
 * layout someone posts in a forum is the same text this app writes — and `parseUiState`'s
 * migration is what makes an older file still open.
 */
export interface BoardFile {
  kind: "stonqs.dashboard";
  version: number;
  dashboard: Dashboard;
}

export function boardToFile(board: Dashboard): string {
  const file: BoardFile = { kind: "stonqs.dashboard", version: UI_VERSION, dashboard: board };
  return JSON.stringify(file, null, 2);
}

/**
 * Reads a board file, giving it a fresh id so importing one twice does not shadow the first.
 * Returns `null` rather than throwing: a wrong file is a message to the user, not a crash.
 */
export function boardFromFile(text: string): Dashboard | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  // A bare board is accepted too: that is what `defaultDashboard.json` holds.
  const file = parsed as Partial<BoardFile> & Partial<Dashboard>;
  const board = (file.dashboard ?? file) as unknown;
  if (!isDashboard(board)) return null;
  const migrated = migrateBoard(board, version((parsed as Partial<BoardFile>).version));
  return { ...migrated, id: newId("d"), widgets: migrated.widgets.map((w) => ({ ...w, id: newId("w") })) };
}
