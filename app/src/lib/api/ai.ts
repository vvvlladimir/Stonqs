/** The assistant: keys, chats, models, turns and the dashboard brief. */

import { Channel } from "@tauri-apps/api/core";
import type {
  AiChat,
  AiEvent,
  AiProvider,
  AiToolMode,
  AiEffort,
  AiUsageTotal,
  ToolDecision,
  ChatMessage,
  DateString,
} from "../types";
import { call, type Source } from "./core";

export const aiApi = {
  /** No command reads a saved key back — only save, delete, and a connected/not-connected status. */
  aiKeySave: (provider: string, key: string) => call<void>("ai_key_save", { provider, key }),
  aiKeyDelete: (provider: string) => call<void>("ai_key_delete", { provider }),
  aiKeyStatus: (provider: string) => call<boolean>("ai_key_status", { provider }),

  aiChatsList: () => call<AiChat[]>("ai_chats_list"),
  aiChatCreate: (title: string) => call<AiChat>("ai_chat_create", { title }),
  aiChatRename: (id: string, title: string) => call<void>("ai_chat_rename", { id, title }),
  aiChatSetMode: (id: string, mode: AiToolMode) => call<void>("ai_chat_set_mode", { id, mode }),
  aiChatSetEffort: (id: string, effort: AiEffort) => call<void>("ai_chat_set_effort", { id, effort }),
  aiChatSetModel: (id: string, model: string) => call<void>("ai_chat_set_model", { id, model }),
  /** Moving a chat to another provider takes its model with it: a model id belongs to the
   *  catalogue it came from, so the host lands the chat on the new provider's default. */
  aiChatSetProvider: (id: string, provider: string) => call<void>("ai_chat_set_provider", { id, provider }),
  /** The providers this build can talk to. A list of what exists, not of what is connected. */
  aiProvidersList: () => call<AiProvider[]>("ai_providers_list"),
  /** What one provider says it offers, best matches first; nothing is filtered out. */
  aiModelsList: (provider: string) => call<string[]>("ai_models_list", { provider }),
  aiChatDelete: (id: string) => call<void>("ai_chat_delete", { id }),
  /** One chat as Markdown, written to a path the user picked — the way to keep a conversation
   *  after deleting the history. */
  aiChatExportSave: (id: string, path: string) => call<void>("ai_chat_export_save", { id, path }),
  aiMessagesList: (chatId: string) => call<ChatMessage[]>("ai_messages_list", { chatId }),
  /** The tools already allowed for the rest of this chat — never the tools that exist. */
  aiGrantsList: (chatId: string) => call<string[]>("ai_grants_list", { chatId }),
  /** Answers one consent card by id. The host looks the call up; this sends nothing else. */
  aiToolDecide: (requestId: string, decision: ToolDecision) =>
    call<void>("ai_tool_decide", { requestId, decision }),
  aiCancel: () => call<void>("ai_cancel"),

  /** What every model has been asked for so far, counted by the provider, never priced here. */
  aiUsageTotals: () => call<AiUsageTotal[]>("ai_usage_totals"),
  /** The dashboard brief: a fixed set of readings, one model call, streamed like a chat turn. */
  aiBrief: (
    from: DateString,
    to: DateString,
    source: Source,
    instructions: string | null,
    language: string,
    /** The tile's own; `null` follows where a new chat begins, and the newest model there. */
    model: { provider: string | null; model: string | null },
    /** The answer's length in output tokens; `null` leaves the provider's own ceiling. */
    maxTokens: number | null,
    onEvent: (event: AiEvent) => void,
  ) => {
    const channel = new Channel<AiEvent>();
    channel.onmessage = onEvent;
    return call<void>("ai_brief", {
      from,
      to,
      source: source ?? null,
      instructions,
      language,
      provider: model.provider,
      model: model.model,
      maxTokens,
      onEvent: channel,
    });
  },

  /**
   * Streams the reply through `onEvent`. The promise settles once dispatched; a rejection means
   * the turn never started, and a mid-turn failure arrives as an `error` event.
   */
  aiSend: (
    chatId: string,
    text: string,
    screen: string | null,
    asOf: string | null,
    onEvent: (event: AiEvent) => void,
  ) => {
    const channel = new Channel<AiEvent>();
    channel.onmessage = onEvent;
    return call<void>("ai_send", { chatId, text, screen, asOf, onEvent: channel });
  },
};
