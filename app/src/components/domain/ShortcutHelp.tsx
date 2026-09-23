import { Trans, useLingui } from "@lingui/react/macro";
import {
  CAPS,
  COMMANDS,
  keyParts,
  type CommandDef,
  type CommandGroup,
  type CommandId,
} from "../../lib/commands";
import { useUiState } from "../../lib/uiState";
import { Banner, Kbd, List, ListRow, Modal, Panel } from "../ui";

/** The list the `?` key opens: every shortcut the app has, read off the one catalogue. */
export function ShortcutHelp({ onClose }: { onClose: () => void }) {
  const { t, i18n } = useLingui();
  const { ui } = useUiState();
  const single = ui.shortcuts.single_keys;

  const groups: Array<{ id: CommandGroup; title: string }> = [
    { id: "general", title: t`General` },
    { id: "navigation", title: t`Navigation` },
    { id: "create", title: t`Create` },
    { id: "data", title: t`Data` },
    { id: "screen", title: t`On a screen` },
  ];
  const ids = Object.keys(COMMANDS) as CommandId[];
  // Keys every dialog, menu and table answers to: part of the controls, not of the catalogue.
  const builtIn: Array<{ label: string; keys: string[][][] }> = [
    { label: t`Close a dialog, a menu or the assistant`, keys: [[[CAPS.esc]]] },
    { label: t`Save a dialog, also from a multi-line field`, keys: [[[CAPS.mod, CAPS.enter]]] },
    { label: t`Move between rows of a table or a menu`, keys: [[["↑"]], [["↓"]]] },
    { label: t`Open the actions of a row`, keys: [[[CAPS.shift, CAPS.f10]]] },
    { label: t`Move to the next or the previous control`, keys: [[[CAPS.tab]], [[CAPS.shift, CAPS.tab]]] },
  ];

  const row = (key: string, label: string, keys: string[][][]) => (
    <ListRow
      key={key}
      title={label}
      end={
        <span className="kbd-list">
          {keys.map((steps, i) => (
            <Kbd key={i} steps={steps} />
          ))}
        </span>
      }
    />
  );

  return (
    <Modal title={t`Keyboard shortcuts`} onClose={onClose} wide>
      <div className="stack">
        {!single && (
          <Banner tone="info">
            <Trans>Single-key shortcuts are turned off in Settings, Keyboard.</Trans>
          </Banner>
        )}
        {groups.map((group) => (
          <Panel key={group.id} title={group.title}>
            <List>
              {ids
                .filter((id) => COMMANDS[id].group === group.id)
                .map((id) => {
                  const def: CommandDef = COMMANDS[id];
                  const bindings = def.keys.filter((b) => single || b.includes("mod"));
                  if (bindings.length === 0) return null;
                  // Nine bindings of one kind read as a range.
                  const shown = id === "fav" ? [bindings[0], bindings[bindings.length - 1]] : bindings;
                  return row(id, i18n._(def.label), shown.map(keyParts));
                })}
            </List>
          </Panel>
        ))}
        <Panel title={t`Everywhere`}>
          <List>{builtIn.map((item) => row(item.label, item.label, item.keys))}</List>
        </Panel>
      </div>
    </Modal>
  );
}
