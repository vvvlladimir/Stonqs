import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import { useUiState } from "../uiState";
import { COMMAND_IDS, commandDef, type ChoiceId, type CommandId } from "./catalog";
import { chordMatches, sequence, typing } from "./keys";
import { createRegistry, type ChoiceOption, type CommandArg, type Handler, type Registry } from "./registry";

/**
 * The React side of the command layer: the one window `keydown` listener, and the hooks by
 * which a component answers a command, publishes a choice or takes the keyboard as a layer.
 *
 * - A bare key never fires while text is being typed and is switched off as a whole by
 *   `UiState::shortcuts.single_keys` (WCAG 2.1.4). A `mod` binding works everywhere.
 * - While a layer is open only `Escape` and the commands it lets `pass` reach anything.
 */

const Context = createContext<Registry | null>(null);

/** How long the second key of `g p` is waited for. */
const SEQUENCE_MS = 1500;
/** Held down, these repeat; everything else runs once per press. */
const REPEATS: readonly CommandId[] = ["periodPrev", "periodNext"];

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
      if (bare && (!single.current || typing(e.target))) {
        pending = null;
        return;
      }

      const now = Date.now();
      const prefix = pending && pending.until > now ? pending.chord : null;
      pending = null;

      for (const id of COMMAND_IDS) {
        const keys = commandDef(id).keys;
        for (let index = 0; index < keys.length; index++) {
          const steps = sequence(keys[index]);
          const matched =
            steps.length === 1
              ? prefix === null && chordMatches(e, steps[0])
              : prefix !== null && keys[index].startsWith(`${prefix} `) && chordMatches(e, steps[1]);
          if (!matched) continue;
          if (e.repeat && !REPEATS.includes(id)) return e.preventDefault();
          if (registry.run(id, index, "key")) {
            e.preventDefault();
            return;
          }
        }
      }

      // The first key of a sequence waits for its second, and types nothing meanwhile.
      if (bare && prefix === null && registry.layers.length === 0) {
        const opener = COMMAND_IDS.flatMap((id) => commandDef(id).keys).find(
          (b) => b.includes(" ") && chordMatches(e, sequence(b)[0]),
        );
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

/** The registry itself: `run`, `pick`, `available`, … — what the palette and the menu bar read. */
export function useRegistry(): Registry | null {
  return useContext(Context);
}

/** Re-renders whenever what answers changes: a registration, a layer, a published choice. */
export function useRegistryVersion(): number {
  const registry = useContext(Context);
  return useSyncExternalStore(registry?.subscribe ?? noSubscribe, registry?.version ?? zero);
}

const noSubscribe = () => () => {};
const zero = () => 0;

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
    return registry.register(id, { handler: ref, label, priority });
  }, [registry, id, enabled, label, priority]);
}

/**
 * `useCommand` as an element, for a screen that declares its actions in JSX beside the button.
 * It also runs an intent left for it: a command with a `screen` pressed elsewhere opened this
 * screen, and the element is what is mounted once the screen can answer.
 */
export function Command({
  id,
  run,
  disabled,
  label,
}: {
  id: CommandId;
  run: (arg?: CommandArg) => void;
  disabled?: boolean;
  label?: string;
}) {
  const registry = useContext(Context);
  useCommand(id, run, { enabled: !disabled, label });
  const latest = useRef(run);
  useLayoutEffect(() => {
    latest.current = run;
  });
  useEffect(() => {
    if (!disabled && registry?.take(id)) latest.current(undefined);
  }, [registry, id, disabled]);
  return null;
}

/**
 * Publishes a choice while mounted. The options are compared by value, so a parent that
 * rebuilds the array on every render does not re-publish it.
 */
export function useChoice(
  id: ChoiceId,
  choice: {
    label: string;
    options: ChoiceOption[];
    value: string | null;
    pick: (value: string) => void;
  } | null,
  priority = 1,
) {
  const registry = useContext(Context);
  const pick = useRef(choice?.pick ?? noPick);
  useLayoutEffect(() => {
    pick.current = choice?.pick ?? noPick;
  });
  const shape = choice ? JSON.stringify([choice.label, choice.options, choice.value]) : null;

  useEffect(() => {
    if (!registry || shape === null) return;
    const [label, options, value] = JSON.parse(shape) as [string, ChoiceOption[], string | null];
    return registry.publish(id, { label, options, value, pick, priority });
  }, [registry, id, shape, priority]);
}

const noPick = () => {};

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
