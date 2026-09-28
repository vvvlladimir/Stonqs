import { useCallback, useReducer } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { CUSTOM_PROVIDER, providerName } from "./kinds";
import { api, ApiError } from "./api";
import { keys, useInvalidate } from "./queries";
import { useNav } from "./nav";
import type { AiEvent, AiUsage, ChatMessage, ToolDecision, ToolParams, UiError } from "./types";
import { useAsOf } from "./asOf";

/** Mirrors `ai/brief.rs::READINGS`; a Rust test pins the two lists together. */
export const BRIEF_READINGS = ["portfolio_overview", "portfolio_performance", "positions_list"];

/** A brand name for built-ins, the user's label for the custom one, or one word of ours if blank. */
export function useProviderName() {
  const { t } = useLingui();
  return (id: string, label?: string) =>
    id === CUSTOM_PROVIDER && !label?.trim() ? t`My provider` : providerName(id, label);
}

/** A tool call waiting on the user. Only `requestId` is ever sent back — see `ai/consent.rs`. */
export interface ToolRequest {
  requestId: string;
  tool: string;
  params: ToolParams;
  /** Whether the call changes data. A write card offers no standing permission — there is none. */
  write: boolean;
  /** The model's sentence on why. Its own words, so it is rendered as text, never as a label. */
  reason: string;
}

/** A reading while the turn runs, shaped like the stored block. */
export interface LiveTool {
  kind: "tool" | "search";
  /** The tool's name, or the search query. */
  name: string;
  /** Why the model wanted it — shown even when the chat runs reads without asking. */
  reason?: string;
  /** What it answered; absent while it is still running. */
  content?: string;
}

/** What the panel shows of the turn in flight. */
interface Turn {
  pending: string;
  /** Kept apart from `pending`: the thinking is shown above the answer and folded away, so the
   *  two must not be concatenated into one stream of text. */
  thinking: string;
  busy: boolean;
  live: LiveTool[];
  request: ToolRequest | null;
  error: UiError | null;
  /** What the *last* question cost, not the chat: a new message starts the count again, the way a
   *  chat's running total belongs in Settings rather than over the input box. */
  usage: AiUsage | null;
}

const IDLE: Turn = {
  pending: "",
  thinking: "",
  busy: false,
  live: [],
  request: null,
  error: null,
  usage: null,
};

/** The summary is persisted with the turn, so every live copy goes when the stored one lands. */
const SETTLED = { busy: false, live: [], request: null, pending: "", thinking: "" } satisfies Partial<Turn>;

type TurnAction = AiEvent | { type: "start" } | { type: "refused"; error: UiError } | { type: "decided" };

function turnReducer(turn: Turn, action: TurnAction): Turn {
  switch (action.type) {
    case "start":
      return { ...IDLE, busy: true };
    case "refused":
      return { ...turn, busy: false, error: action.error };
    case "decided":
      return { ...turn, request: null };
    case "text":
      return { ...turn, pending: turn.pending + action.text };
    case "tool_requested":
      return {
        ...turn,
        request: {
          requestId: action.request_id,
          tool: action.tool,
          params: action.params,
          write: action.write,
          reason: action.reason,
        },
      };
    case "tool_running":
      return {
        ...turn,
        request: null,
        live: [...turn.live, { kind: "tool", name: action.tool, reason: action.reason }],
      };
    case "tool_finished":
      // The last unfinished entry for this tool: the model may call one twice in a turn, and the
      // second answer belongs to the second call.
      return { ...turn, live: fill(turn.live, action.tool, action.content) };
    case "reasoning":
      return { ...turn, thinking: turn.thinking + action.text };
    case "searching":
      return { ...turn, live: [...turn.live, { kind: "search", name: action.query }] };
    case "usage":
      // Assigned, never added: the host sends the turn's running total, so a step that arrives
      // twice or out of order still leaves the right number on screen.
      return { ...turn, usage: action.usage };
    case "done":
      return { ...turn, ...SETTLED };
    case "error":
      return { ...turn, ...SETTLED, error: action.error };
  }
}

/** The stream, kept out of the query cache: only its persisted result belongs there. Failures stay `UiError`. */
export function useChatSend(chatId: string | null) {
  const client = useQueryClient();
  // Taken at send time, not at render: the screen behind the panel is part of the question.
  const { screen } = useNav();
  // The date lens, for the same reason: a question asked over a past portfolio is about it.
  const { date: asOf, isToday } = useAsOf();
  const invalidate = useInvalidate();
  const [turn, dispatch] = useReducer(turnReducer, IDLE);

  const send = useCallback(
    async (text: string) => {
      if (!chatId) return;
      dispatch({ type: "start" });

      // Shown at once; the host writes the same turn, so the refetch replaces it with an identical row.
      const optimistic: ChatMessage = {
        id: `pending-${Date.now()}`,
        chat_id: chatId,
        role: "user",
        blocks: [{ type: "text", text }],
        created_at: new Date().toISOString(),
      };
      client.setQueryData<ChatMessage[]>(keys.aiMessages(chatId), (current) => [
        ...(current ?? []),
        optimistic,
      ]);

      try {
        await api.aiSend(chatId, text, screen, isToday ? null : asOf, (event: AiEvent) => {
          dispatch(event);
          // Always, not only on success: the user's own turn was persisted before the call, so
          // even a failed reply leaves the chat further along than the cache thinks.
          if (event.type === "done" || event.type === "error") {
            invalidate(keys.aiMessages(chatId), keys.aiChats(), keys.aiGrants(chatId));
          }
        });
      } catch (e) {
        // The command refused before the turn started (panel off, no key saved). Nothing was
        // written, so the optimistic line has to go.
        dispatch({
          type: "refused",
          error: e instanceof ApiError ? e.detail : { code: "internal", message: String(e) },
        });
        client.setQueryData<ChatMessage[]>(keys.aiMessages(chatId), (current) =>
          (current ?? []).filter((message) => message.id !== optimistic.id),
        );
      }
    },
    [chatId, client, invalidate, screen, asOf, isToday],
  );

  const decide = useCallback((requestId: string, decision: ToolDecision) => {
    // Cleared first: the card is answered whatever the host makes of it, and a second click
    // on a card already spent would be answering nothing.
    dispatch({ type: "decided" });
    void api.aiToolDecide(requestId, decision).catch(() => dispatch({ type: "decided" }));
  }, []);

  const stop = useCallback(() => {
    void api.aiCancel();
  }, []);

  return { ...turn, send, decide, stop };
}

function fill(live: LiveTool[], tool: string, content: string): LiveTool[] {
  let index = -1;
  for (let i = live.length - 1; i >= 0; i--) {
    const entry = live[i];
    if (entry.kind === "tool" && entry.name === tool && entry.content === undefined) {
      index = i;
      break;
    }
  }
  if (index < 0) return live;
  return live.map((entry, i) => (i === index ? { ...entry, content } : entry));
}
