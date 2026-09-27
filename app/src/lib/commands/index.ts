/** The command layer (ADR-0072). Import from `lib/commands` only. */
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
