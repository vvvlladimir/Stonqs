/** The assistant panel: keys, chats, models, usage, grants. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import { keys } from "./keys";

/** Whether a key is saved for the given provider — never the key itself. */
export function useAiKeyStatus(provider: string) {
  return useQuery({ queryKey: keys.aiKeyStatus(provider), queryFn: () => api.aiKeyStatus(provider) });
}

export function useAiChats() {
  return useQuery({ queryKey: keys.aiChats(), queryFn: api.aiChatsList });
}

/** Without an open chat the hook stays disabled rather than listing nothing. */
export function useAiMessages(chatId: string | null) {
  return useQuery({
    queryKey: keys.aiMessages(chatId ?? undefined),
    queryFn: () => api.aiMessagesList(chatId!),
    enabled: chatId !== null,
  });
}

/** The providers this build can talk to. Compiled in, so it never goes stale while the app runs. */
export function useAiProviders() {
  return useQuery({ queryKey: keys.aiProviders(), queryFn: api.aiProvidersList, staleTime: Infinity });
}

/** Once per provider per run, keyed by provider. */
export function useAiModels(provider: string | null, enabled: boolean) {
  return useQuery({
    queryKey: keys.aiModels(provider ?? undefined),
    queryFn: () => api.aiModelsList(provider!),
    enabled: enabled && provider !== null,
    staleTime: Infinity,
  });
}

/** What every model has cost so far. Refetched with the chat list: a finished turn moves both. */
export function useAiUsage() {
  return useQuery({ queryKey: keys.aiUsage(), queryFn: api.aiUsageTotals });
}

/** The tools this chat may already read without asking again. */
export function useAiGrants(chatId: string | null) {
  return useQuery({
    queryKey: keys.aiGrants(chatId ?? undefined),
    queryFn: () => api.aiGrantsList(chatId!),
    enabled: chatId !== null,
  });
}
