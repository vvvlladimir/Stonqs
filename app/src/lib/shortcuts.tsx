import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useUiState } from "./uiState";

/**
 * The keyboard layer: one `keydown` listener for the whole window, a catalogue of what a key
 * does, and a stack of layers (a dialog, the assistant) that decides who hears `Escape`.
 *
 * - A binding is `mod+k` (⌘ on macOS, Ctrl elsewhere), a bare key (`n`, `?`, `[`) or a sequence
 *   (`g p`). A letter is matched by what it types when that is Latin and by its physical
 *   position otherwise, so ⌘K still works with a Cyrillic layout active.
 * - A bare key never fires while text is being typed and is switched off as a whole by
 *   `UiState::shortcuts.single_keys` (WCAG 2.1.4). A `mod` binding works everywhere.
 * - While a layer is open only `Escape` and the commands it lets `pass` reach anything.
 */

export const IS_MAC =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);

export type CommandGroup = "general" | "navigation" | "screen";

export interface CommandDef {
  keys: string[];
  label: MessageDescriptor;
  group: CommandGroup;
  /** Kept out of the palette: the palette lists the screens themselves instead. */
  hidden?: boolean;
}

/* eslint-disable lingui/no-unlocalized-strings -- bindings and ids; the labels are the text */
export const COMMANDS = {
  palette: { keys: ["mod+k"], label: msg`Command palette`, group: "general" },
  help: { keys: ["?", "mod+/"], label: msg`Keyboard shortcuts`, group: "general" },
  search: { keys: ["mod+f", "/"], label: msg`Search on this screen`, group: "general" },
  newTransaction: { keys: ["mod+n"], label: msg`New transaction`, group: "general" },
  refresh: { keys: ["mod+r"], label: msg`Refresh quotes and rates`, group: "general" },
  ai: { keys: ["mod+j"], label: msg`Open or close the AI assistant`, group: "general" },
  lock: { keys: ["mod+shift+l"], label: msg`Lock profile`, group: "general" },
  settings: { keys: ["mod+,"], label: msg`Settings`, group: "navigation", hidden: true },
  fav: {
    keys: ["mod+1", "mod+2", "mod+3", "mod+4", "mod+5", "mod+6", "mod+7", "mod+8", "mod+9"],
    label: msg`Overview and favourites, in order`,
    group: "navigation",
    hidden: true,
  },
  goDashboard: { keys: ["g o"], label: msg`Go to Overview`, group: "navigation", hidden: true },
  goPositions: { keys: ["g p"], label: msg`Go to Positions`, group: "navigation", hidden: true },
  goTransactions: { keys: ["g t"], label: msg`Go to Transactions`, group: "navigation", hidden: true },
  goAccounts: { keys: ["g a"], label: msg`Go to Accounts`, group: "navigation", hidden: true },
  goSecurities: { keys: ["g i"], label: msg`Go to Instruments`, group: "navigation", hidden: true },
  goWatchlist: { keys: ["g w"], label: msg`Go to Watchlist`, group: "navigation", hidden: true },
  new: { keys: ["n"], label: msg`Create on this screen`, group: "screen" },
  periodPrev: { keys: ["["], label: msg`Previous period`, group: "screen" },
  periodNext: { keys: ["]"], label: msg`Next period`, group: "screen" },
} satisfies Record<string, CommandDef>;
/* eslint-enable lingui/no-unlocalized-strings */

export type CommandId = keyof typeof COMMANDS;

// ---------------------------------------------------------------------------------------------
// Bindings

interface Chord {
  mod: boolean;
  shift: boolean;
  key: string;
}

function chord(text: string): Chord {
  const parts = text.split("+");
  // `mod+shift+,` splits cleanly; a bare `+` is not a binding this catalogue uses.
  const key = parts[parts.length - 1];
  return { mod: parts.includes("mod"), shift: parts.includes("shift"), key };
}

function sequence(binding: string): Chord[] {
  return binding.split(" ").map(chord);
}

/** Keys whose character already implies Shift, so the modifier is not compared for them. */
const SHIFTED = new Set(["?"]);
/* eslint-disable lingui/no-unlocalized-strings -- `KeyboardEvent.code` values */
const CODES: Record<string, string> = {
  "[": "BracketLeft",
  "]": "BracketRight",
  "/": "Slash",
  ",": "Comma",
  "?": "Slash",
};
/* eslint-enable lingui/no-unlocalized-strings */

function keyMatches(e: KeyboardEvent, key: string): boolean {
  if (/^[a-z]$/.test(key)) {
    // Latin layouts by the letter typed (Dvorak, AZERTY); any other by the key's position.
    return /^[a-z]$/i.test(e.key) ? e.key.toLowerCase() === key : e.code === `Key${key.toUpperCase()}`;
  }
  if (/^[0-9]$/.test(key)) return e.code === `Digit${key}`;
  // By position only with the Shift state the character implies: plain `/` is not `?`.
  return e.key === key || (e.code === CODES[key] && e.shiftKey === SHIFTED.has(key));
}

function chordMatches(e: KeyboardEvent, c: Chord): boolean {
  const mod = IS_MAC ? e.metaKey : e.ctrlKey;
  const other = IS_MAC ? e.ctrlKey : e.metaKey;
  if (mod !== c.mod || other || e.altKey) return false;
  if (!SHIFTED.has(c.key) && e.shiftKey !== c.shift) return false;
  return keyMatches(e, c.key);
}

/** How a key is printed on the keyboard of this platform. */
export const CAPS = {
  mod: IS_MAC ? "⌘" : "Ctrl",
  shift: IS_MAC ? "⇧" : "Shift",
  enter: IS_MAC ? "↵" : "Enter",
  esc: "Esc",
  tab: "Tab",
  f10: "F10",
};
const ARIA = { mod: IS_MAC ? "Meta" : "Control", shift: "Shift", menuMod: "CmdOrCtrl" };

const KEY_NAMES: Record<string, string> = { enter: CAPS.enter };

/** A binding as the keys printed on the keyboard: `["⌘", "K"]`, `["Ctrl", "Shift", "L"]`. */
export function keyParts(binding: string): string[][] {
  return sequence(binding).map((c) => [
    ...(c.mod ? [CAPS.mod] : []),
    ...(c.shift ? [CAPS.shift] : []),
    KEY_NAMES[c.key] ?? c.key.toUpperCase(),
  ]);
}

/** The first binding of a command as one short string, for a tooltip: `⌘K`, `Ctrl+K`, `G P`. */
export function keyHint(id: CommandId): string {
  const binding = COMMANDS[id].keys[0];
  return keyParts(binding)
    .map((parts) => parts.join(IS_MAC ? "" : "+"))
    .join(" ");
}

/** One binding in `aria-keyshortcuts` notation, or `undefined` for a sequence it cannot express. */
export function ariaBinding(binding: string): string | undefined {
  if (binding.includes(" ")) return undefined;
  const c = chord(binding);
  return [c.mod ? ARIA.mod : null, c.shift ? ARIA.shift : null, c.key.toUpperCase()]
    .filter(Boolean)
    .join("+");
}

/** `aria-keyshortcuts` for a control that a command also triggers. */
export function ariaKeys(id: CommandId): string {
  return COMMANDS[id].keys.flatMap((b) => ariaBinding(b) ?? []).join(" ");
}

/** A `mod` binding in the notation of the native menu, or `undefined` for one it cannot carry. */
export function accelerator(binding: string): string | undefined {
  const [c, ...rest] = sequence(binding);
  if (rest.length > 0 || !c.mod) return undefined;
  return [ARIA.menuMod, c.shift ? ARIA.shift : null, c.key.toUpperCase()].filter(Boolean).join("+");
}

/** Typing into this element owns the bare keys: a field, a list box, an open menu. */
function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  if (el.isContentEditable) return true;
  if (el.closest('[role="menu"], [role="listbox"], [role="combobox"], [role="slider"]')) return true;
  if (el.tagName === "TEXTAREA" || el.tagName === "SELECT") return true;
  if (el.tagName !== "INPUT") return false;
  const type = (el as HTMLInputElement).type;
  return !["checkbox", "radio", "button", "submit", "reset", "range", "color", "file"].includes(type);
}

// ---------------------------------------------------------------------------------------------
// Registry

type Handler = (index: number) => void;

interface Entry {
  handler: { current: Handler };
  label?: string;
  priority: number;
  order: number;
}

interface Layer {
  escape: { current: () => void };
  pass: { current: readonly CommandId[] };
}

interface Registry {
  register: (id: CommandId, entry: Entry) => () => void;
  pushLayer: (layer: Layer) => () => void;
  run: (id: CommandId, index?: number, source?: "key" | "menu" | "palette") => boolean;
  /** Commands that would run now, with the label a screen gave its own. */
  available: () => Array<{ id: CommandId; label?: string }>;
}

const Context = createContext<Registry | null>(null);

/** How long the second key of `g p` is waited for. */
const SEQUENCE_MS = 1500;
/** A native menu item and the webview can both see one keystroke; the second is dropped. */
const ECHO_MS = 300;

type Live = Registry & {
  layers: Layer[];
};

function createRegistry(): Live {
  const entries = new Map<CommandId, Entry[]>();
  const layers: Layer[] = [];
  let order = 0;
  let last: { id: CommandId; source: string; at: number } | null = null;

  const top = (id: CommandId) => {
    const list = entries.get(id);
    if (!list || list.length === 0) return undefined;
    return list.reduce((a, b) =>
      b.priority > a.priority || (b.priority === a.priority && b.order > a.order) ? b : a,
    );
  };
  const allowed = (id: CommandId) => {
    const layer = layers[layers.length - 1];
    return !layer || layer.pass.current.includes(id);
  };

  const value: Live = {
    layers,
    register(id, entry) {
      const stamped = { ...entry, order: ++order };
      entries.set(id, [...(entries.get(id) ?? []), stamped]);
      return () =>
        entries.set(
          id,
          (entries.get(id) ?? []).filter((e) => e !== stamped),
        );
    },
    pushLayer(layer) {
      layers.push(layer);
      return () => {
        const at = layers.indexOf(layer);
        if (at >= 0) layers.splice(at, 1);
      };
    },
    run(id, index = 0, source = "key") {
      const entry = top(id);
      if (!entry || !allowed(id)) return false;
      const now = Date.now();
      if (last && last.id === id && last.source !== source && now - last.at < ECHO_MS) return true;
      last = { id, source, at: now };
      entry.handler.current(index);
      return true;
    },
    available() {
      return (Object.keys(COMMANDS) as CommandId[])
        .filter((id) => !(COMMANDS[id] as CommandDef).hidden && allowed(id))
        .flatMap((id) => {
          const entry = top(id);
          return entry ? [{ id, label: entry.label }] : [];
        });
    },
  };
  return value;
}

export function ShortcutsProvider({ children }: { children: ReactNode }) {
  const singleKeys = useUiState().ui.shortcuts.single_keys;
  const single = useRef(singleKeys);
  useLayoutEffect(() => {
    single.current = singleKeys;
  });

  const [registry] = useState(createRegistry);

  useEffect(() => {
    let pending: { chord: string; until: number } | null = null;

    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.keyCode === 229) return;

      if (e.key === "Escape") {
        const layer = registry.layers[registry.layers.length - 1];
        if (!layer) return;
        e.preventDefault();
        layer.escape.current();
        return;
      }

      const bare = !e.metaKey && !e.ctrlKey && !e.altKey;
      const bareAllowed = single.current && !typing(e.target);
      if (bare && !bareAllowed) {
        pending = null;
        return;
      }

      const now = Date.now();
      const prefix = pending && pending.until > now ? pending.chord : null;
      pending = null;

      for (const id of Object.keys(COMMANDS) as CommandId[]) {
        const keys = (COMMANDS[id] as CommandDef).keys;
        for (let index = 0; index < keys.length; index++) {
          const steps = sequence(keys[index]);
          const matched =
            steps.length === 1
              ? prefix === null && chordMatches(e, steps[0])
              : prefix !== null && keys[index].startsWith(`${prefix} `) && chordMatches(e, steps[1]);
          if (!matched) continue;
          if (e.repeat && id !== "periodPrev" && id !== "periodNext") return e.preventDefault();
          if (registry.run(id, index, "key")) {
            e.preventDefault();
            return;
          }
        }
      }

      // The first key of a sequence waits for its second, and types nothing meanwhile.
      if (bare && prefix === null && registry.layers.length === 0) {
        const opener = Object.values(COMMANDS)
          .flatMap((c) => c.keys)
          .find((b) => b.includes(" ") && chordMatches(e, sequence(b)[0]));
        if (opener) {
          pending = { chord: opener.split(" ")[0], until: now + SEQUENCE_MS };
          e.preventDefault();
        }
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [registry]);

  return <Context.Provider value={registry}>{children}</Context.Provider>;
}

/** Runs a command by id — what the palette and the native menu call. */
export function useRunCommand() {
  const registry = useContext(Context);
  return registry ?? { run: () => false, available: () => [] };
}

/**
 * Answers a command while mounted. A screen's own answer outranks the shell's (`priority` 0),
 * so `mod+n` on Transactions opens the form in place instead of navigating to it.
 */
export function useCommand(
  id: CommandId,
  handler: Handler,
  options: { enabled?: boolean; label?: string; priority?: number } = {},
) {
  const registry = useContext(Context);
  const ref = useRef(handler);
  useLayoutEffect(() => {
    ref.current = handler;
  });
  const { enabled = true, label, priority = 1 } = options;

  useEffect(() => {
    if (!registry || !enabled) return;
    return registry.register(id, { handler: ref, label, priority, order: 0 });
  }, [registry, id, enabled, label, priority]);
}

/** `useCommand` as an element, for a screen that declares its actions in JSX beside the button. */
export function Command({
  id,
  run,
  disabled,
  label,
}: {
  id: CommandId;
  run: () => void;
  disabled?: boolean;
  label?: string;
}) {
  useCommand(id, run, { enabled: !disabled, label });
  return null;
}

/**
 * Takes `Escape` and the keyboard while mounted: a dialog, the assistant, the palette. The
 * newest layer answers first, so `Escape` closes one thing at a time.
 */
export function useLayer(
  onEscape: () => void,
  options: { pass?: readonly CommandId[]; active?: boolean } = {},
) {
  const registry = useContext(Context);
  const escape = useRef(onEscape);
  const pass = useRef(options.pass ?? NONE);
  useLayoutEffect(() => {
    escape.current = onEscape;
    pass.current = options.pass ?? NONE;
  });
  const { active = true } = options;

  // Keyed on nothing that changes per render: re-pushing would lift an older layer above a newer one.
  useEffect(() => {
    if (!active) return;
    if (registry) return registry.pushLayer({ escape, pass });
    // Outside the provider (a test harness, a screen drawn before the shell) `Escape` still closes.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) escape.current();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [registry, active]);
}

const NONE: readonly CommandId[] = [];

// ---------------------------------------------------------------------------------------------
// Intents: a command that opens something on a screen that is not mounted yet.

const intents = new Set<string>();

/** Leaves a request for the next screen to mount: `mod+n` from Overview opens Transactions' form. */
export function requestIntent(name: "newTransaction") {
  intents.add(name);
}

/** Consumes a request once; read in a `useState` initialiser so it opens with the screen. */
export function takeIntent(name: "newTransaction"): boolean {
  return intents.delete(name);
}
