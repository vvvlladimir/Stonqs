/** The assistant: chats, blocks, consent and what a turn cost. */

import type { UiError } from "./primitives";
/** How a chat treats tool calls; `AUTO` covers reads only — see `ai/tools.rs`. */
export type AiToolMode = "ASK" | "AUTO";

/** How hard the model is asked to think in this chat. */
export type AiEffort = "LOW" | "MEDIUM" | "HIGH";

/** One provider this build can talk to. `connected` is whether a turn would get as far as the
 * network — a saved key for a built-in provider, an address and a model for the custom one,
 * whose key is optional. Never anything about the key itself.
 *
 * `label` is empty for the built-in ones, whose names this side already knows, and carries the
 * user's own words for the custom one. */
export interface AiProvider {
  id: string;
  connected: boolean;
  label: string;
}

export interface AiChat {
  id: string;
  title: string;
  provider: string;
  model: string;
  tool_mode: AiToolMode;
  effort: AiEffort;
  created_at: string;
  updated_at: string;
}

export type AiRole = "user" | "model";

/** One turn's content. Rendered directly, never through a Lingui macro — this is the model's or
 * the user's own text, not a UI string the app authored. */
export type AiBlock =
  | { type: "text"; text: string }
  | { type: "tool_call"; id: string; name: string; args_json: string; signature?: string | null }
  | { type: "tool_result"; call_id: string; content: string }
  /** A search the provider ran itself; there is nothing to send back, only to show. */
  | { type: "web_search"; query: string }
  /** The model's summary of its own reasoning. Shown, stored, never sent back to the model. */
  | { type: "reasoning"; text: string; signature?: string | null };

export interface ChatMessage {
  id: string;
  chat_id: string;
  role: AiRole;
  blocks: AiBlock[];
  created_at: string;
}

/** What a tool was asked for, as values. The sentence around them is written here, not by the
 * host and never by the model — see `components/domain/aiTools.ts`. */
export type ToolParams = Record<string, string>;

/** The three answers a consent card can give. `session` is remembered for this chat only. */
export type ToolDecision = "once" | "session" | "always" | "deny";

/**
 * What a request cost, as the provider counted it. Counts, not money: the app never prices them,
 * because a price list compiled into the binary is wrong by the provider's next release.
 *
 * `cached_tokens` is part of `input_tokens` and `reasoning_tokens` part of `output_tokens` —
 * a provider that counts neither reports zero for both.
 */
export interface AiUsage {
  input_tokens: number;
  cached_tokens: number;
  output_tokens: number;
  reasoning_tokens: number;
}

/** Everything spent on one model. A deleted chat's requests still count — see ADR-0041. */
export interface AiUsageTotal {
  provider: string;
  model: string;
  /** Requests, not turns: one answer that called tools cost several. */
  requests: number;
  usage: AiUsage;
  first_at: string;
  last_at: string;
}

/**
 * Streamed over the `ai_send` channel — see `lib/ai.ts`. A failure carries a `UiError`, not a
 * sentence: the host ships the code and this side writes the words, as everywhere else.
 */
export type AiEvent =
  | { type: "text"; text: string }
  /** The turn is blocked until `aiToolDecide` answers this `request_id`. */
  | {
      type: "tool_requested";
      request_id: string;
      tool: string;
      params: ToolParams;
      /** A call that changes the portfolio: confirmed every time, never for a whole chat. */
      write: boolean;
      /** The model's own line on why it wants this. Shown beside the values, never instead. */
      reason: string;
    }
  | { type: "tool_running"; tool: string; reason: string }
  | { type: "tool_finished"; tool: string; content: string }
  | { type: "searching"; query: string }
  /** A chunk of the reasoning summary, while the model is still thinking. */
  | { type: "reasoning"; text: string }
  /** What the turn has cost so far — a running total, so it is assigned and never added up. */
  | { type: "usage"; usage: AiUsage }
  | { type: "done" }
  | { type: "error"; error: UiError };
