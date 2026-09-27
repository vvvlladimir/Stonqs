import { useCallback, useState } from "react";
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

/** The stream, kept out of the query cache: only its persisted result belongs there. Failures stay `UiError`. */
export function useChatSend(chatId: string | null) {
  const client = useQueryClient();
  // Taken at send time, not at render: the screen behind the panel is part of the question.
  const { screen } = useNav();
  // The date lens, for the same reason: a question asked over a past portfolio is about it.
  const { date: asOf, isToday } = useAsOf();
  const invalidate = useInvalidate();
  const [pending, setPending] = useState("");
  // Kept apart from `pending`: the thinking is shown above the answer and folded away, so the
  // two must not be concatenated into one stream of text.
  const [thinking, setThinking] = useState("");
  const [busy, setBusy] = useState(false);
  const [live, setLive] = useState<LiveTool[]>([]);
  const [request, setRequest] = useState<ToolRequest | null>(null);
  const [error, setError] = useState<UiError | null>(null);
  // What the *last* question cost, not the chat: a new message starts the count again, the way
  // a chat's running total belongs in Settings rather than over the input box.
  const [usage, setUsage] = useState<AiUsage | null>(null);

  const send = useCallback(
    async (text: string) => {
      if (!chatId) return;
      setPending("");
      setThinking("");
      setError(null);
      setLive([]);
      setRequest(null);
      setUsage(null);
      setBusy(true);

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

      const settle = () => {
        setBusy(false);
        setLive([]);
        setRequest(null);
        setPending("");
        // The summary is persisted with the turn, so the live copy goes when the stored one lands.
        setThinking("");
        // Always, not only on success: the user's own turn was persisted before the call, so
        // even a failed reply leaves the chat further along than the cache thinks.
        invalidate(keys.aiMessages(chatId), keys.aiChats(), keys.aiGrants(chatId));
      };

      try {
        await api.aiSend(chatId, text, screen, isToday ? null : asOf, (event: AiEvent) => {
          switch (event.type) {
            case "text":
              setPending((current) => current + event.text);
              break;
            case "tool_requested":
              setRequest({
                requestId: event.request_id,
                tool: event.tool,
                params: event.params,
                write: event.write,
                reason: event.reason,
              });
              break;
            case "tool_running":
              setRequest(null);
              setLive((current) => [...current, { kind: "tool", name: event.tool, reason: event.reason }]);
              break;
            case "tool_finished":
              // The last unfinished entry for this tool: the model may call one twice in a turn,
              // and the second answer belongs to the second call.
              setLive((current) => fill(current, event.tool, event.content));
              break;
            case "reasoning":
              setThinking((current) => current + event.text);
              break;
            case "searching":
              setLive((current) => [...current, { kind: "search", name: event.query }]);
              break;
            case "usage":
              // Assigned, never added: the host sends the turn's running total, so a step
              // that arrives twice or out of order still leaves the right number on screen.
              setUsage(event.usage);
              break;
            case "done":
              settle();
              break;
            case "error":
              setError(event.error);
              settle();
              break;
          }
        });
      } catch (e) {
        // The command refused before the turn started (panel off, no key saved). Nothing was
        // written, so the optimistic line has to go.
        setBusy(false);
        setError(e instanceof ApiError ? e.detail : { code: "internal", message: String(e) });
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
    setRequest(null);
    void api.aiToolDecide(requestId, decision).catch(() => setRequest(null));
  }, []);

  const stop = useCallback(() => {
    void api.aiCancel();
  }, []);

  return { pending, thinking, busy, live, request, error, usage, send, decide, stop };
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
