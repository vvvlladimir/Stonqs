/** Everything the host persists about how the app is set up. */

import type { UserPeriod } from "./periods";
import type { AiEffort } from "./ai";
export interface AppSettings {
  /** Which build's reading of the stored file applies. The host migrates; nothing here acts on it. */
  version: number;
  auto_refresh_on_start: boolean;
  refresh_min_interval_hours: number;
  last_refresh: string | null;
  /** UI language preference: "system" or a locale tag. See `lib/i18n`. */
  language: string;
  /** The period axis; edited through its own commands, never through `settings_save`. */
  periods: UserPeriod[];
  hidden_presets: string[];
  /** Opaque frontend-owned UI layout state. */
  ui: unknown;
  /** Whether the AI panel is offered at all. A provider key can exist while this is off. */
  ai_enabled: boolean;
  /** Whether the provider may search the web: a question typed here then leaves the machine. */
  ai_web_search: boolean;
  /** Whether the panel shows the model's summary of its own reasoning. Off by default. */
  ai_reasoning: boolean;
  /** Where a new chat starts (e.g. `"openai"`), also the keychain account name. The chat
   * footer's provider switch writes it too. */
  ai_provider: string;
  /** The model last picked in a chat, per provider. Absent, a chat starts on the smallest tier.
   * Written by the chat commands only; `settings_save` keeps what is stored. */
  ai_models: Record<string, string>;
  /** Model ids the user added per provider, offered in the chat's picker after the provider's own. */
  ai_extra_models: Record<string, string[]>;
  /** The thinking effort last picked in a chat, where the next chat starts. */
  ai_effort: AiEffort;
  /** The server the user points the app at themselves. Empty until configured. */
  ai_custom: AiCustomProvider;
  /** Market-data sources switched away from their default; edited through `market_source_switch`. */
  market_sources: Record<string, boolean>;
  /** Whether the owner has said where data may come from. Until then every source is off
   * whatever `market_sources` says, and nothing is fetched. Sealed by `market_sources_confirm`. */
  sources_configured: boolean;
}

/** Which shape of API a server speaks. `OPENAI_CHAT` is what "OpenAI-compatible" means outside
 * OpenAI itself — `POST {base}/chat/completions`, spoken by every gateway, proxy and local
 * runner. `OPENAI_RESPONSES` is OpenAI's own newer shape; `ANTHROPIC` is `/v1/messages`. */
export type AiWire = "OPENAI_CHAT" | "OPENAI_RESPONSES" | "ANTHROPIC" | "GEMINI";

export interface AiCustomProvider {
  /** What to call it in the pickers. The user's own words. */
  label: string;
  /** Up to and including the version segment: `https://openrouter.ai/api/v1`. */
  base_url: string;
  wire: AiWire;
  /** Typed rather than picked: a server behind a base URL need not offer a catalogue. */
  model: string;
}

// AI assistant
