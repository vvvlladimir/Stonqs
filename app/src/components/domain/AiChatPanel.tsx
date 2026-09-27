import { useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Trans, useLingui } from "@lingui/react/macro";
import { CaretLeftIcon, SparkleIcon, XIcon } from "@phosphor-icons/react";
import { useProviderName } from "../../lib/ai";
import { usePointerDrag } from "../../lib/pointerDrag";
import { useLayer, type CommandId } from "../../lib/commands";
import { AI_PANEL_MAX, AI_PANEL_MIN, useUiState } from "../../lib/uiState";
import { useAiChats, useAiProviders } from "../../lib/queries";
import { useDialogFocus } from "../ui";
import { ActiveChat } from "./aiChat/ActiveChat";
import { ChatList } from "./aiChat/ChatList";

const AI_PASS: readonly CommandId[] = ["ai"];

/**
 * A global drawer, not a screen — mounted once at the shell level next to `TooltipLayer`.
 *
 * Nothing in here carries a `data-tip`: the panel is a conversation, and a hover bubble over the
 * text of one is noise. What a control does is on the control, and what it is for is its
 * `aria-label`.
 */
export function AiChatPanel({ onClose }: { onClose: () => void }) {
  const { t } = useLingui();
  const [chatId, setChatId] = useState<string | null>(null);
  const width = usePanelWidth();
  const chats = useAiChats();
  const chat = chats.data?.find((entry) => entry.id === chatId);
  const providers = useAiProviders();
  const name = useProviderName();

  const panel = useRef<HTMLDivElement>(null);
  // `mod+j` passes through so the key that opened the panel also closes it.
  useLayer(onClose, { pass: AI_PASS });
  useDialogFocus(panel);

  return createPortal(
    <div className="ai-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={panel}
        className="ai-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t`AI assistant`}
        // A custom property, not an inline width: below 700px the panel is full-screen and the
        // stylesheet simply does not read it, so one number cannot fight the narrow layout.
        style={{ "--ai-panel-w": `${width.value}px` } as React.CSSProperties}
      >
        <div
          className="ai-panel__grip"
          role="separator"
          aria-orientation="vertical"
          aria-label={t`Resize the panel`}
          onPointerDown={width.onPointerDown}
        />
        <header className="ai-head">
          {chatId ? (
            <button
              type="button"
              className="iconbtn iconbtn--sm"
              aria-label={t`Back to the chats`}
              onClick={() => setChatId(null)}
            >
              <CaretLeftIcon />
            </button>
          ) : (
            <span className="ai-mark" aria-hidden>
              <SparkleIcon weight="fill" />
            </span>
          )}
          <div className="ai-head__text">
            <span className="ai-head__title">{chat ? chat.title : <Trans>Vibe</Trans>}</span>
            <span className="ai-head__sub">
              {chat ? (
                `${name(chat.provider, providers.data?.find((p) => p.id === chat.provider)?.label)} · ${chat.model}`
              ) : (
                <Trans>Portfolio assistant</Trans>
              )}
            </span>
          </div>
          <button
            type="button"
            className="iconbtn iconbtn--sm ai-head__close"
            aria-label={t`Close`}
            onClick={onClose}
          >
            <XIcon />
          </button>
        </header>
        {chatId ? <ActiveChat chatId={chatId} /> : <ChatList onOpen={setChatId} />}
      </div>
    </div>,
    document.body,
  );
}

/**
 * The drag on the panel's left edge. The width follows the pointer in local state and is only
 * written to `UiState` when the gesture ends — a settings write per pixel would be one file
 * write per pixel.
 */
function usePanelWidth() {
  const { ui, save } = useUiState();
  const [dragged, setDragged] = useState<number | null>(null);
  const from = useRef(ui.ai_panel_width);

  const onPointerDown = usePointerDrag({
    onStart: () => {
      from.current = ui.ai_panel_width;
      setDragged(ui.ai_panel_width);
    },
    // The panel is docked to the right, so dragging its edge left makes it wider.
    onMove: (dx) => setDragged(clamp(from.current - dx)),
    onEnd: (commit) => {
      setDragged((current) => {
        if (commit && current !== null && current !== ui.ai_panel_width) {
          save((ui) => ({ ...ui, ai_panel_width: current }));
        }
        return null;
      });
    },
  });

  return { value: dragged ?? ui.ai_panel_width, onPointerDown };
}

function clamp(width: number): number {
  return Math.min(Math.max(Math.round(width), AI_PANEL_MIN), AI_PANEL_MAX);
}
