import type { ScreenId } from "../nav";
import { COMMAND_IDS, commandDef, type ChoiceId, type CommandId } from "./catalog";

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

/** Mounted commands, published choices and the layer stack. `subscribe` and `version` are fields
 *  rather than methods: `useSyncExternalStore` is handed them on their own. */
export class Registry {
  readonly layers: Layer[] = [];
  private readonly commands = new Map<CommandId, Stamped<CommandEntry>[]>();
  private readonly choiceMap = new Map<ChoiceId, Stamped<ChoiceEntry>[]>();
  private readonly intents = new Map<CommandId, number>();
  private readonly listeners = new Set<() => void>();
  private nav: Navigator | null = null;
  private order = 0;
  private count = 0;
  private last: { id: CommandId; source: RunSource; at: number } | null = null;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };
  version = () => this.count;

  register = (id: CommandId, entry: CommandEntry) => this.add(this.commands, id, entry);
  publish = (id: ChoiceId, entry: ChoiceEntry) => this.add(this.choiceMap, id, entry);

  pushLayer(layer: Layer) {
    this.layers.push(layer);
    this.changed();
    return () => {
      const at = this.layers.indexOf(layer);
      if (at >= 0) this.layers.splice(at, 1);
      this.changed();
    };
  }

  navigate(next: Navigator) {
    const moved = this.nav?.screen !== next.screen;
    this.nav = next;
    if (moved) this.changed();
  }

  /** Whether running `id` now would do something. */
  can(id: CommandId): boolean {
    if (!this.allowed(id)) return false;
    return winner(this.commands.get(id)) !== undefined || this.elsewhere(id) !== null;
  }

  label(id: CommandId) {
    return winner(this.commands.get(id))?.label;
  }

  run(id: CommandId, arg?: CommandArg, source: RunSource = "key"): boolean {
    if (!this.allowed(id)) return false;
    const entry = winner(this.commands.get(id));
    const screen = entry ? null : this.elsewhere(id);
    if (!entry && !screen) return false;
    const now = Date.now();
    const last = this.last;
    if (last && last.id === id && last.source !== source && now - last.at < ECHO_MS) return true;
    this.last = { id, source, at: now };
    if (entry) entry.handler.current(arg);
    else if (screen && this.nav) {
      this.intents.set(id, now);
      this.nav.go(screen);
    }
    return true;
  }

  /** Consumes the intent a command left for this screen, once. */
  take(id: CommandId): boolean {
    const at = this.intents.get(id);
    this.intents.delete(id);
    return at !== undefined && Date.now() - at < INTENT_MS;
  }

  /** Commands the palette may offer now, with the label their screen gave them. */
  available(): Array<{ id: CommandId; label?: string }> {
    return COMMAND_IDS.filter((id) => !commandDef(id).hidden && this.can(id)).map((id) => ({
      id,
      label: this.label(id),
    }));
  }

  choice(id: ChoiceId): Choice | undefined {
    return winner(this.choiceMap.get(id));
  }

  choices(): Array<[ChoiceId, Choice]> {
    return [...this.choiceMap.keys()].flatMap((id) => {
      const choice = winner(this.choiceMap.get(id));
      return choice ? [[id, choice] as [ChoiceId, Choice]] : [];
    });
  }

  pick(id: ChoiceId, value: string) {
    winner(this.choiceMap.get(id))?.pick.current(value);
  }

  private changed() {
    this.count += 1;
    for (const listener of this.listeners) listener();
  }

  private add<K, T>(map: Map<K, Stamped<T>[]>, key: K, entry: T) {
    const stamped = { ...entry, order: ++this.order };
    map.set(key, [...(map.get(key) ?? []), stamped]);
    this.changed();
    return () => {
      map.set(
        key,
        (map.get(key) ?? []).filter((e) => e !== stamped),
      );
      this.changed();
    };
  }

  private allowed(id: CommandId) {
    const layer = this.layers[this.layers.length - 1];
    return !layer || layer.pass.current.includes(id);
  }

  /** A command nobody on this screen answers, which its own screen would. */
  private elsewhere(id: CommandId) {
    const screen = commandDef(id).screen;
    return screen !== undefined && this.nav !== null && this.nav.screen !== screen ? screen : null;
  }
}

export function createRegistry() {
  return new Registry();
}
