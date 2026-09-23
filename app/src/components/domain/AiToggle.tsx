import { useLingui } from "@lingui/react/macro";
import { SparkleIcon } from "@phosphor-icons/react";

import { useSettings } from "../../lib/queries";

/**
 * Dock button that opens the panel; hidden entirely when the panel is disabled in Settings.
 *
 * Deliberately not in `AiChatPanel.tsx`: the panel is loaded only once it is opened, and one
 * module holding both would pull the whole conversation — markdown renderer included — into the
 * chunk that draws the dock.
 */
export function AiToggle({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { t } = useLingui();
  const settings = useSettings();
  if (!settings.data?.ai_enabled) return null;
  return (
    <button type="button" className="iconbtn ai-toggle" data-tip={t`AI assistant`} onClick={onToggle}>
      <SparkleIcon weight={open ? "fill" : "regular"} />
    </button>
  );
}
