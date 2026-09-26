import { Trans, useLingui } from "@lingui/react/macro";
import { useUiState } from "../../lib/uiState";
import { keyHint, useRegistry } from "../../lib/commands";
import { CheckField, Form, ListRow, Panel } from "../../components/ui";

/** Keyboard shortcuts: the bare keys can be switched off, and the full list is one press away. */
export function KeyboardPanel() {
  const { t } = useLingui();
  const { ui, save } = useUiState();
  const registry = useRegistry();

  return (
    <Panel title={t`Keyboard`} info={t`Shortcuts for the commands used most, and how to turn some off.`}>
      <Form>
        <CheckField
          label={t`Single-key shortcuts`}
          hint={t`Keys without ⌘ or Ctrl, such as N for a new item or G then P for Positions. Turn them off if they are pressed by accident or interfere with speech input.`}
          checked={ui.shortcuts.single_keys}
          onChange={(single_keys) => save((ui) => ({ ...ui, shortcuts: { ...ui.shortcuts, single_keys } }))}
        />
      </Form>
      <ListRow
        box
        wrap
        title={t`All keyboard shortcuts`}
        sub={t`Also opens with ${keyHint("help")} anywhere in the app.`}
        end={
          <button type="button" className="btn" onClick={() => registry?.run("help", undefined, "palette")}>
            <Trans>Show</Trans>
          </button>
        }
      />
    </Panel>
  );
}
