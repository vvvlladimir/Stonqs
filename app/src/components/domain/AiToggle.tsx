import { useLingui } from "@lingui/react/macro";
import { SparkleIcon } from "@phosphor-icons/react";

import { useSettings } from "../../lib/queries";
import { ariaKeys, keyHint } from "../../lib/commands";

/** Kept apart from `AiChatPanel` so the dock does not pull the lazy panel into its chunk. */
export function AiToggle({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { t } = useLingui();
  const settings = useSettings();
  if (!settings.data?.ai_enabled) return null;
  return (
    <button
      type="button"
      className="iconbtn ai-toggle"
      aria-label={t`AI assistant`}
      aria-expanded={open}
      aria-keyshortcuts={ariaKeys("ai")}
      data-tip={`${t`AI assistant`} · ${keyHint("ai")}`}
      onClick={onToggle}
    >
      <SparkleIcon weight={open ? "fill" : "regular"} />
    </button>
  );
}
