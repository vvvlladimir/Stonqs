/** The chat footer's controls: tool mode, effort, provider, model and the token count. */

import { type ReactNode } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  CaretDownIcon,
  CircleNotchIcon,
  CpuIcon,
  GaugeIcon,
  LightningIcon,
  LockSimpleIcon,
  PlugsConnectedIcon,
} from "@phosphor-icons/react";
import type { AiEffort, AiProvider, AiToolMode, AiUsage } from "../../lib/types";
import { useMenu, type MenuItem } from "../ui/Menu";
import { formatDecimal } from "../../lib/format";
import { useProviderName } from "../../lib/ai";

/** A footer control: a pill that opens the app's own menu. Never a native `<select>` — the
 * platform one opens a list as long as the catalogue and lands under the pointer, which is how
 * a four-item choice ended up looking like a directory. */
function Pill({
  icon,
  label,
  items,
  ariaLabel,
  on,
}: {
  icon: ReactNode;
  label: string;
  items: MenuItem[];
  ariaLabel: string;
  on?: boolean;
}) {
  const menu = useMenu();
  return (
    <>
      <button
        type="button"
        className={on ? "ai-pill ai-pill--on" : "ai-pill"}
        aria-haspopup="menu"
        aria-label={ariaLabel}
        onClick={(e) => menu.openFrom(ariaLabel, items, e.currentTarget)}
      >
        {icon}
        <span className="ai-pill__label">{label}</span>
        <CaretDownIcon className="ai-pill__caret" />
      </button>
      {menu.node}
    </>
  );
}

/**
 * The chat's standing answer about tools: ask each time, or read freely. One control, and the
 * card's "Allow all" flips the same switch — two ways to say one thing, never two settings.
 *
 * It is per chat on purpose: a conversation opened to poke around does not make the next one
 * permissive. What it covers is reads; a write tool is confirmed on its own whatever this says.
 */
export function ToolModeToggle({
  mode,
  onChange,
}: {
  mode: AiToolMode;
  onChange: (mode: AiToolMode) => void;
}) {
  const { t } = useLingui();
  const auto = mode === "AUTO";
  return (
    <button
      type="button"
      className={auto ? "ai-pill ai-pill--on" : "ai-pill"}
      aria-pressed={auto}
      aria-label={auto ? t`Reading your data without asking` : t`Asking before reading your data`}
      onClick={() => onChange(auto ? "ASK" : "AUTO")}
    >
      {auto ? <LightningIcon weight="fill" /> : <LockSimpleIcon />}
      <span className="ai-pill__label">{auto ? <Trans>Auto</Trans> : <Trans>Ask first</Trans>}</span>
    </button>
  );
}

/** How hard this chat asks the model to think. Three steps, because that is what every provider
 * exposes under its own name; what a model without reasoning does with it is the adapter's. */
export function EffortPicker({
  effort,
  onChange,
}: {
  effort: AiEffort;
  onChange: (effort: AiEffort) => void;
}) {
  const { t } = useLingui();
  const labels: Record<AiEffort, string> = {
    LOW: t`Fast`,
    MEDIUM: t`Balanced`,
    HIGH: t`Thorough`,
  };
  const items = (["LOW", "MEDIUM", "HIGH"] as AiEffort[]).map((value) => ({
    label: labels[value],
    checked: value === effort,
    onSelect: () => onChange(value),
  }));
  return <Pill icon={<GaugeIcon />} label={labels[effort]} items={items} ariaLabel={t`Thinking`} />;
}

/**
 * Which provider answers this chat. Per chat rather than an app setting, for the reason the model
 * and the tool mode are: two conversations open side by side may be answered by different ones,
 * and switching here must not reach into the next chat.
 *
 * Every provider this build can talk to is listed, connected or not — a picker that hides the
 * other one when only one key is saved reads as no choice at all. One without a key cannot be
 * chosen, and says why rather than failing on the next send.
 */
export function ProviderPicker({
  provider,
  providers,
  onChange,
}: {
  provider: string;
  providers: AiProvider[];
  onChange: (provider: string) => void;
}) {
  const { t } = useLingui();
  const name = useProviderName();
  // The chat's own is always listed: a chat must never show a provider it is not on, whatever
  // this build knows or the keychain says.
  const known = providers.some((entry) => entry.id === provider)
    ? providers
    : [{ id: provider, connected: true, label: "" }, ...providers];
  if (known.length < 2) return null;

  const items = known.map((entry) => ({
    label: entry.connected ? name(entry.id, entry.label) : t`${name(entry.id, entry.label)} — no key saved`,
    checked: entry.id === provider,
    disabled: !entry.connected,
    onSelect: () => onChange(entry.id),
  }));
  return (
    <Pill
      icon={<PlugsConnectedIcon />}
      label={name(provider, known.find((entry) => entry.id === provider)?.label)}
      items={items}
      ariaLabel={t`Which provider answers`}
    />
  );
}

/**
 * Which model answers this chat: the three the provider's catalogue puts at the top of each of
 * its tiers (`ai/models/`). The chat's current one is always an option, even when the list
 * could not be fetched and even when it is not one of the three.
 */
export function ModelPicker({
  model,
  models,
  loading,
  onChange,
}: {
  model: string;
  models: string[];
  /** The catalogue is a network call; until it lands the menu holds the chat's model alone,
   * which would otherwise read as "this provider offers one model". */
  loading?: boolean;
  onChange: (model: string) => void;
}) {
  const { t } = useLingui();
  const options = models.includes(model) ? models : [model, ...models];
  const items: MenuItem[] = options.map((value) => ({
    label: value,
    checked: value === model,
    onSelect: () => onChange(value),
  }));
  if (loading) items.push({ label: t`Loading the model list…`, disabled: true, onSelect: () => {} });
  return <Pill icon={<CpuIcon />} label={model} items={items} ariaLabel={t`Which model answers`} />;
}

/**
 * What the question being answered has cost, in tokens the provider counted — never in money:
 * a price list compiled into the app is wrong by the provider's next release, and the running
 * total per model lives in Settings anyway.
 *
 * It covers one turn, not the chat: a new message starts the count again. Providers report at
 * different moments — OpenAI only once a request finishes, so a turn that calls tools counts up
 * as it goes and a plain answer lands its figure at the end. The spinner says which is happening
 * rather than showing a guessed number: nothing here is estimated from the text on screen.
 */
export function TokenCount({ usage, busy }: { usage: AiUsage | null; busy: boolean }) {
  const { t } = useLingui();
  if (!usage && !busy) return null;

  const count = (value: number) => formatDecimal(String(value), { digits: 0 });
  return (
    <span className="ai-usage" aria-label={t`Tokens this answer cost`}>
      {busy && <CircleNotchIcon className="spin" />}
      <ArrowUpIcon />
      <span className="num">{usage ? count(usage.input_tokens) : "—"}</span>
      <ArrowDownIcon />
      <span className="num">{usage ? count(usage.output_tokens) : "—"}</span>
    </span>
  );
}
