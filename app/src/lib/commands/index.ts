/**
 * The command layer (ADR-0072): one catalogue of what the app can do, one registry of who
 * answers it now, and the keyboard, palette and menu bar reading both. Import from
 * `lib/commands`, never from a file inside it.
 */
export {
  COMMAND_IDS,
  COMMANDS,
  commandDef,
  type ChoiceId,
  type CommandDef,
  type CommandGroup,
  type CommandId,
} from "./catalog";
export { ariaBinding, ariaKeys, CAPS, IS_MAC, keyHint, keyParts, menuAccelerator } from "./keys";
export type { Choice, ChoiceOption, CommandArg, Registry, RunSource } from "./registry";
export {
  Command,
  ShortcutsProvider,
  useChoice,
  useCommand,
  useLayer,
  useRegistry,
  useRegistryVersion,
} from "./react";
