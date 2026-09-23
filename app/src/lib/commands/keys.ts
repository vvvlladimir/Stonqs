import { COMMANDS, commandDef, type CommandId } from "./catalog";

/**
 * Bindings: parsing, matching a `KeyboardEvent`, and printing them for the platform.
 * A letter is matched by what it types on a Latin layout and by its physical position on any
 * other, so ⌘K still works with a Cyrillic layout active.
 */

export const IS_MAC =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);

export interface Chord {
  mod: boolean;
  shift: boolean;
  key: string;
}

export function chord(text: string): Chord {
  const parts = text.split("+");
  // `mod+shift+,` splits cleanly; a bare `+` is not a binding the catalogue uses.
  const key = parts[parts.length - 1];
  return { mod: parts.includes("mod"), shift: parts.includes("shift"), key };
}

export function sequence(binding: string): Chord[] {
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

export function chordMatches(e: KeyboardEvent, c: Chord): boolean {
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
  const binding = commandDef(id).keys[0];
  if (!binding) return "";
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
  return commandDef(id)
    .keys.flatMap((b) => ariaBinding(b) ?? [])
    .join(" ");
}

/**
 * The accelerator a menu item shows: the binding at `index` when the command has one per
 * argument (the favourites), else its first `mod` binding. Bare keys never go to the menu,
 * where they would fire while typing.
 */
export function menuAccelerator(id: CommandId, index?: number): string | undefined {
  const keys = COMMANDS[id].keys as string[];
  const binding = index !== undefined ? keys[index] : keys.find((b) => b.startsWith("mod+"));
  if (!binding) return undefined;
  const [c, ...rest] = sequence(binding);
  if (rest.length > 0 || !c.mod) return undefined;
  return [ARIA.menuMod, c.shift ? ARIA.shift : null, c.key.toUpperCase()].filter(Boolean).join("+");
}

/** Typing into this element owns the bare keys: a field, a list box, an open menu. */
export function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  if (el.isContentEditable) return true;
  if (el.closest('[role="menu"], [role="listbox"], [role="combobox"], [role="slider"]')) return true;
  if (el.tagName === "TEXTAREA" || el.tagName === "SELECT") return true;
  if (el.tagName !== "INPUT") return false;
  const type = (el as HTMLInputElement).type;
  return !["checkbox", "radio", "button", "submit", "reset", "range", "color", "file"].includes(type);
}
