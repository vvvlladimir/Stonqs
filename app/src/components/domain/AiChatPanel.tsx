import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { save } from "@tauri-apps/plugin-dialog";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  CaretLeftIcon,
  ChatCircleDotsIcon,
  PaperPlaneRightIcon,
  PlusIcon,
  SparkleIcon,
  StopIcon,
  XIcon,
} from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { useChatSend, useProviderName } from "../../lib/ai";
import { formatDateTime } from "../../lib/format";
import { usePointerDrag } from "../../lib/pointerDrag";
import { AI_PANEL_MAX, AI_PANEL_MIN, useUiState } from "../../lib/uiState";
import {
  keys,
  useAiChats,
  useAiGrants,
  useAiMessages,
  useAiModels,
  useAiProviders,
  useInvalidate,
} from "../../lib/queries";
import type { AiBlock, AiChat, AiEffort, AiToolMode, ChatMessage, UiError } from "../../lib/types";
import {
  EffortPicker,
  GrantedTools,
  ModelPicker,
  ProviderPicker,
  Steps,
  ToolConsent,
  ToolModeToggle,
  TokenCount,
} from "./AiTools";
import { liveSteps, storedSteps, type Step } from "./aiSteps";
import {
  Banner,
  ErrorText,
  Field,
  FormDialog,
  ListRow,
  Markdown,
  Pending,
  QueryError,
  useMenu,
  useUiErrorText,
  type MenuItem,
} from "../ui";

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

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div className="ai-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
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
          save({ ...ui, ai_panel_width: current });
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

function ChatList({ onOpen }: { onOpen: (id: string) => void }) {
  const { t } = useLingui();
  const chats = useAiChats();
  const invalidate = useInvalidate();
  const providers = useAiProviders();
  // A chat carries its own provider, so what blocks a new one is having no key anywhere — not
  // the state of the one a new chat happens to start on.
  const connected = providers.data?.some((provider) => provider.connected) ?? false;
  const menu = useMenu();
  // The chat being renamed, held with its current title so the dialog opens on what it says now.
  const [renaming, setRenaming] = useState<{ id: string; title: string } | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: () => api.aiChatCreate(t`New chat`),
    onSuccess: (chat) => {
      invalidate(keys.aiChats());
      onOpen(chat.id);
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.aiChatDelete(id),
    onSuccess: () => invalidate(keys.aiChats()),
  });
  const rename = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) => api.aiChatRename(id, title),
    onSuccess: () => {
      setRenaming(null);
      invalidate(keys.aiChats());
    },
  });

  // Markdown, not CSV: a chat is prose with readings in it, and the panel already renders it as
  // markdown — the file is what is on screen plus the tool calls behind it.
  const exportChat = async (chat: AiChat) => {
    const path = await save({
      defaultPath: `${fileName(chat.title)}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!path) return;
    setSaveError(null);
    try {
      await api.aiChatExportSave(chat.id, path);
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  };

  if (chats.isError) return <QueryError error={chats.error} />;
  if (!chats.data) return <Pending />;

  // A chat names itself from its first message; renaming by hand wins from then on, which is why
  // the host never retitles a chat that already has history.
  const itemsFor = (chat: AiChat): MenuItem[] => [
    { label: t`Rename…`, onSelect: () => setRenaming({ id: chat.id, title: chat.title }) },
    { label: t`Export as Markdown…`, onSelect: () => void exportChat(chat) },
    { label: t`Delete`, danger: true, onSelect: () => remove.mutate(chat.id) },
  ];

  return (
    <>
      <div className="ai-body ai-body--list">
        {!connected && (
          <Banner tone="info">
            <Trans>Add your API key in Settings → AI assistant to start chatting.</Trans>
          </Banner>
        )}
        {chats.data.length === 0 ? (
          <div className="ai-opener">
            <span className="ai-mark ai-mark--big" aria-hidden>
              <SparkleIcon weight="fill" />
            </span>
            <h3>
              <Trans>No chats yet</Trans>
            </h3>
            <p className="muted">
              <Trans>Ask about performance, allocation, a single holding — anything on screen.</Trans>
            </p>
          </div>
        ) : (
          <div className="ai-chats">
            {chats.data.map((chat: AiChat) => (
              <ListRow
                key={chat.id}
                box
                lead={<ChatCircleDotsIcon />}
                title={chat.title}
                sub={formatDateTime(chat.updated_at)}
                onClick={() => onOpen(chat.id)}
                {...menu.row(chat.id, itemsFor(chat))}
                actions={menu.button(chat.id, itemsFor(chat))}
              />
            ))}
          </div>
        )}
        {saveError && (
          <ErrorText>
            <Trans>Could not save the file: {saveError}</Trans>
          </ErrorText>
        )}
        {menu.node}
        {renaming && (
          <FormDialog
            title={t`Rename chat`}
            onClose={() => setRenaming(null)}
            onSubmit={() => rename.mutate({ id: renaming.id, title: renaming.title.trim() })}
            busy={rename.isPending}
            error={rename.error}
            ready={renaming.title.trim() !== ""}
          >
            <Field label={t`Title`}>
              <input
                autoFocus
                value={renaming.title}
                onChange={(e) => setRenaming({ ...renaming, title: e.target.value })}
              />
            </Field>
          </FormDialog>
        )}
      </div>
      {/* Floating over the list rather than heading it: starting a chat is what this screen is
          for, and the button stays under the thumb however far the history has been scrolled. */}
      <button
        type="button"
        className="ai-fab"
        disabled={create.isPending || !connected}
        onClick={() => create.mutate()}
      >
        <PlusIcon weight="bold" />
        <Trans>New chat</Trans>
      </button>
    </>
  );
}

/** A title is the user's own words and can hold anything; a file name cannot. */
function fileName(title: string): string {
  const cleaned = title.replace(/[\\/:*?"<>|]/g, " ").trim();
  return cleaned === "" ? "chat" : cleaned.slice(0, 60);
}

function ActiveChat({ chatId }: { chatId: string }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const chats = useAiChats();
  const messages = useAiMessages(chatId);
  const grants = useAiGrants(chatId);
  const chat = chats.data?.find((entry) => entry.id === chatId);
  const mode: AiToolMode = chat?.tool_mode ?? "ASK";
  const providers = useAiProviders();
  // Keyed to the chat's own provider and only asked for once the chat is open: the list is a
  // network call, and the panel's list of chats should not pay for it.
  const models = useAiModels(chat?.provider ?? null, chat !== undefined);

  const setMode = useMutation({
    mutationFn: (next: AiToolMode) => api.aiChatSetMode(chatId, next),
    onSuccess: () => invalidate(keys.aiChats()),
  });
  const setEffort = useMutation({
    mutationFn: (next: AiEffort) => api.aiChatSetEffort(chatId, next),
    onSuccess: () => invalidate(keys.aiChats(), keys.settings()),
  });
  const setModel = useMutation({
    mutationFn: (next: string) => api.aiChatSetModel(chatId, next),
    onSuccess: () => invalidate(keys.aiChats(), keys.settings()),
  });
  const setProvider = useMutation({
    mutationFn: (next: string) => api.aiChatSetProvider(chatId, next),
    onSuccess: () => invalidate(keys.aiChats(), keys.settings()),
  });
  const { pending, thinking, busy, live, request, error, usage, send, decide, stop } = useChatSend(chatId);
  const [text, setText] = useState("");

  const stored = useMemo(() => conversation(messages.data ?? []), [messages.data]);
  const running = busy ? liveSteps(live, thinking, pending) : [];
  const empty = stored.length === 0 && !busy;

  const submit = (value: string) => {
    const asked = value.trim();
    if (!asked || busy) return;
    setText("");
    send(asked);
  };

  const scroller = useScrollToEnd([stored.length, running.length, pending, thinking, request]);

  return (
    <>
      <div className="ai-body" ref={scroller}>
        {messages.isError && <QueryError error={messages.error} />}
        {empty && <Opener onPick={submit} />}
        {stored.map((part) =>
          part.kind === "steps" ? (
            <Steps key={part.key} steps={part.steps} />
          ) : (
            <Said key={part.key} role={part.role} text={part.text} />
          ),
        )}
        {running.length > 0 && <Steps steps={running} />}
        {busy && pending && <Said role="model" text={pending} streaming />}
        {request && <ToolConsent request={request} onDecide={decide} />}
        {busy && !pending && running.length === 0 && !request && <Working />}
        {error && <TurnError error={error} />}
        {grants.data && grants.data.length > 0 && <GrantedTools tools={grants.data} />}
      </div>
      <div className="ai-compose">
        <TokenCount usage={usage} busy={busy} />
        <Composer
          text={text}
          busy={busy}
          placeholder={t`Ask about your portfolio…`}
          onChange={setText}
          onSubmit={() => submit(text)}
          onStop={stop}
        />
        <div className="ai-foot">
          <ToolModeToggle mode={mode} onChange={(next) => setMode.mutate(next)} />
          <EffortPicker effort={chat?.effort ?? "MEDIUM"} onChange={(next) => setEffort.mutate(next)} />
          {chat && (
            <ProviderPicker
              provider={chat.provider}
              providers={providers.data ?? []}
              onChange={(next) => setProvider.mutate(next)}
            />
          )}
          {chat && (
            <ModelPicker
              model={chat.model}
              models={models.data ?? []}
              loading={models.isPending}
              onChange={(next) => setModel.mutate(next)}
            />
          )}
        </div>
      </div>
    </>
  );
}

/** The input box and the one button beside it, which sends or stops depending on what the turn
 * is doing. The box grows with the text up to a few lines and then scrolls: a conversation is
 * the point of the panel, not the draft. */
function Composer({
  text,
  busy,
  placeholder,
  onChange,
  onSubmit,
  onStop,
}: {
  text: string;
  busy: boolean;
  placeholder: string;
  onChange: (text: string) => void;
  onSubmit: () => void;
  onStop: () => void;
}) {
  const { t } = useLingui();
  const box = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [text]);

  return (
    <div className="ai-input">
      <textarea
        ref={box}
        rows={1}
        value={text}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            onSubmit();
          }
        }}
      />
      {busy ? (
        <button type="button" className="ai-send ai-send--stop" aria-label={t`Stop`} onClick={onStop}>
          <StopIcon weight="fill" />
        </button>
      ) : (
        <button
          type="button"
          className="ai-send"
          disabled={!text.trim()}
          aria-label={t`Send`}
          onClick={onSubmit}
        >
          <PaperPlaneRightIcon weight="fill" />
        </button>
      )}
    </div>
  );
}

/** A chat with nothing in it yet. The openers are questions this app can actually answer, and
 * they are sent as the user's own words — nothing is prefilled into the model's prompt. */
function Opener({ onPick }: { onPick: (text: string) => void }) {
  const { t } = useLingui();
  const asks = [
    t`How did my portfolio do this year?`,
    t`What am I most exposed to?`,
    t`Where am I off my target allocation?`,
    t`How much did I pay in fees?`,
  ];
  return (
    <div className="ai-opener">
      <span className="ai-mark ai-mark--big" aria-hidden>
        <SparkleIcon weight="fill" />
      </span>
      <h3>
        <Trans>Ask about your portfolio</Trans>
      </h3>
      <p className="muted">
        <Trans>It reads the figures behind the screen you are on, and asks before it does.</Trans>
      </p>
      <div className="ai-opener__asks">
        {asks.map((ask) => (
          <button key={ask} type="button" className="ai-ask-chip" onClick={() => onPick(ask)}>
            {ask}
          </button>
        ))}
      </div>
    </div>
  );
}

/** The gap between sending and the first thing coming back. Three dots rather than the app's
 * spinner: this is the model taking its time, not the app loading. */
function Working() {
  const { t } = useLingui();
  return (
    <p className="ai-dots" aria-label={t`Working…`}>
      <span />
      <span />
      <span />
    </p>
  );
}

/** One side's words. The user's sit in a bubble, the model's do not: an answer is the page,
 * not a quote on it. */
function Said({ role, text, streaming }: { role: string; text: string; streaming?: boolean }) {
  const cls = `ai-said ai-said--${role}${streaming ? " ai-said--streaming" : ""}`;
  return (
    <div className={cls}>
      <Markdown>{text}</Markdown>
    </div>
  );
}

/** What the panel draws, in order: said things, and runs of steps between them. */
type Part =
  { kind: "said"; key: string; role: string; text: string } | { kind: "steps"; key: string; steps: Step[] };

/**
 * The stored turns flattened into that order. Readings are grouped across turns on purpose: a
 * provider that answers one tool call per turn would otherwise draw a dozen separate cards for
 * one question, which is the answer buried rather than explained.
 *
 * A turn holding only tool results is not a message of its own — the result belongs under the
 * call that asked for it, which is joined by `call_id` across the whole chat rather than by
 * position inside one turn.
 */
function conversation(messages: ChatMessage[]): Part[] {
  const results = resultsById(messages);
  const parts: Part[] = [];

  for (const message of messages) {
    const steps = storedSteps(message.blocks, results, message.id);
    if (steps.length > 0) {
      const last = parts[parts.length - 1];
      if (last?.kind === "steps") last.steps = [...last.steps, ...steps];
      else parts.push({ kind: "steps", key: `steps:${message.id}`, steps });
    }
    const text = message.blocks
      .filter((block) => block.type === "text")
      .map((block) => (block.type === "text" ? block.text : ""))
      .join("\n\n")
      .trim();
    if (text) parts.push({ kind: "said", key: `said:${message.id}`, role: message.role, text });
  }
  return parts;
}

function resultsById(messages: ChatMessage[]): Map<string, AiBlock> {
  const found = new Map<string, AiBlock>();
  for (const message of messages) {
    for (const block of message.blocks) {
      if (block.type === "tool_result") found.set(block.call_id, block);
    }
  }
  return found;
}

/** Follows the conversation while it is at the bottom, and stops following the moment the user
 * scrolls up to read something — an answer being read must not be yanked away by the next token. */
function useScrollToEnd(deps: unknown[]) {
  const ref = useRef<HTMLDivElement>(null);
  const stick = useRef(true);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onScroll = () => {
      stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    };
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    const el = ref.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
    // The conversation's own length and the streaming text: what changed is what to follow.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return ref;
}

/** A failed turn wears the app's one red line, from the host's code — never a raw message. */
function TurnError({ error }: { error: UiError }) {
  return <ErrorText>{useUiErrorText(error)}</ErrorText>;
}
