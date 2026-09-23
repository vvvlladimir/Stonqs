import type { ScreenId } from "../nav";
import { COMMAND_IDS, commandDef, type ChoiceId, type CommandId } from "./catalog";

/**
 * What is answering right now: the commands mounted components registered, the choices they
 * published, and the stack of layers (a dialog, the assistant) that decides who hears `Escape`.
 * Plain data with a `subscribe`, so the keyboard, the palette and the menu bar read one state.
 */

/** A favourite's index, a screen id — whatever the one command needs to know which one. */
export type CommandArg = string | number | undefined;
export type Handler = (arg: CommandArg) => void;
type Ref<T> = { current: T };

export interface CommandEntry {
  handler: Ref<Handler>;
  /** What the command is called where it is answered: `new` is "New alert" on Alerts. */
  label?: string;
  /** The shell answers at 0; a screen at 1, so its own answer wins while it is mounted. */
  priority: number;
}

export interface ChoiceOption {
  value: string;
  label: string;
}

export interface Choice {
  /** What is being chosen, as a noun: "Period", "Data source". */
  label: string;
  options: ChoiceOption[];
  value: string | null;
}

export interface ChoiceEntry extends Choice {
  pick: Ref<(value: string) => void>;
  priority: number;
}

export interface Layer {
  escape: Ref<() => void>;
  /** Commands that still run while this layer is on top. */
  pass: Ref<readonly CommandId[]>;
}

/** How the registry opens the screen a command belongs to; set by the shell. */
export interface Navigator {
  screen: ScreenId;
  go: (screen: ScreenId) => void;
}

export type RunSource = "key" | "menu" | "palette";

/** A native menu item and the webview can both see one keystroke; the second is dropped. */
const ECHO_MS = 300;
/** An intent the screen did not pick up by then was for a screen that could not answer it. */
const INTENT_MS = 5000;

type Stamped<T> = T & { order: number };

function winner<T extends { priority: number; order: number }>(list: T[] | undefined): T | undefined {
  if (!list || list.length === 0) return undefined;
  return list.reduce((a, b) =>
    b.priority > a.priority || (b.priority === a.priority && b.order > a.order) ? b : a,
  );
}

export function createRegistry() {
  const commands = new Map<CommandId, Stamped<CommandEntry>[]>();
  const choices = new Map<ChoiceId, Stamped<ChoiceEntry>[]>();
  const layers: Layer[] = [];
  const intents = new Map<CommandId, number>();
  const listeners = new Set<() => void>();
  let nav: Navigator | null = null;
  let order = 0;
  let version = 0;
  let last: { id: CommandId; source: RunSource; at: number } | null = null;

  const changed = () => {
    version += 1;
    for (const listener of listeners) listener();
  };

  function add<K, T>(map: Map<K, Stamped<T>[]>, key: K, entry: T) {
    const stamped = { ...entry, order: ++order };
    map.set(key, [...(map.get(key) ?? []), stamped]);
    changed();
    return () => {
      map.set(
        key,
        (map.get(key) ?? []).filter((e) => e !== stamped),
      );
      changed();
    };
  }

  const allowed = (id: CommandId) => {
    const layer = layers[layers.length - 1];
    return !layer || layer.pass.current.includes(id);
  };

  /** A command nobody on this screen answers, which its own screen would. */
  const elsewhere = (id: CommandId) => {
    const screen = commandDef(id).screen;
    return screen !== undefined && nav !== null && nav.screen !== screen ? screen : null;
  };

  const registry = {
    layers,

    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    version: () => version,

    register: (id: CommandId, entry: CommandEntry) => add(commands, id, entry),
    publish: (id: ChoiceId, entry: ChoiceEntry) => add(choices, id, entry),

    pushLayer(layer: Layer) {
      layers.push(layer);
      changed();
      return () => {
        const at = layers.indexOf(layer);
        if (at >= 0) layers.splice(at, 1);
        changed();
      };
    },

    navigate(next: Navigator) {
      const moved = nav?.screen !== next.screen;
      nav = next;
      if (moved) changed();
    },

    /** Whether running `id` now would do something. */
    can(id: CommandId): boolean {
      if (!allowed(id)) return false;
      return winner(commands.get(id)) !== undefined || elsewhere(id) !== null;
    },

    label: (id: CommandId) => winner(commands.get(id))?.label,

    run(id: CommandId, arg?: CommandArg, source: RunSource = "key"): boolean {
      if (!allowed(id)) return false;
      const entry = winner(commands.get(id));
      const screen = entry ? null : elsewhere(id);
      if (!entry && !screen) return false;
      const now = Date.now();
      if (last && last.id === id && last.source !== source && now - last.at < ECHO_MS) return true;
      last = { id, source, at: now };
      if (entry) entry.handler.current(arg);
      else if (screen && nav) {
        intents.set(id, now);
        nav.go(screen);
      }
      return true;
    },

    /** Consumes the intent a command left for this screen, once. */
    take(id: CommandId): boolean {
      const at = intents.get(id);
      intents.delete(id);
      return at !== undefined && Date.now() - at < INTENT_MS;
    },

    /** Commands the palette may offer now, with the label their screen gave them. */
    available(): Array<{ id: CommandId; label?: string }> {
      return COMMAND_IDS.filter((id) => !commandDef(id).hidden && registry.can(id)).map((id) => ({
        id,
        label: registry.label(id),
      }));
    },

    choice(id: ChoiceId): Choice | undefined {
      return winner(choices.get(id));
    },

    choices(): Array<[ChoiceId, Choice]> {
      return [...choices.keys()].flatMap((id) => {
        const choice = winner(choices.get(id));
        return choice ? [[id, choice] as [ChoiceId, Choice]] : [];
      });
    },

    pick(id: ChoiceId, value: string) {
      winner(choices.get(id))?.pick.current(value);
    },
  };
  return registry;
}

export type Registry = ReturnType<typeof createRegistry>;
