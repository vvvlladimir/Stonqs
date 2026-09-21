import { useState, type ReactNode } from "react";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  BrainIcon,
  CaretDownIcon,
  CircleNotchIcon,
  CpuIcon,
  DatabaseIcon,
  GaugeIcon,
  GlobeIcon,
  LightningIcon,
  LockSimpleIcon,
  PencilSimpleIcon,
  PlugsConnectedIcon,
  ShieldCheckIcon,
} from "@phosphor-icons/react";
import type { AiEffort, AiProvider, AiToolMode, AiUsage } from "../../lib/types";
import type { ToolDecision, ToolParams } from "../../lib/types";
import type { ToolRequest } from "../../lib/ai";
import { Markdown } from "../ui/Markdown";
import { useMenu, type MenuItem } from "../ui/Menu";
import { PARAM_LABELS, TOOL_LABELS, paramEntries } from "./aiToolLabels";
import type { Step } from "./aiSteps";
import { formatDecimal } from "../../lib/format";
import { useProviderName } from "../../lib/ai";

function useToolName() {
  const { i18n } = useLingui();
  return (tool: string) => (TOOL_LABELS[tool] ? i18n._(TOOL_LABELS[tool]) : tool);
}

function Values({ params }: { params: ToolParams }) {
  const { i18n } = useLingui();
  const entries = paramEntries(params);
  if (entries.length === 0) return null;
  return (
    <ul className="ai-ask__values">
      {entries.map(([key, value]) => (
        <li key={key}>
          <span className="muted">{PARAM_LABELS[key] ? i18n._(PARAM_LABELS[key]) : key}</span> {value}
        </li>
      ))}
    </ul>
  );
}

/**
 * The question in the flow of the chat, not a dialog: the turn is paused behind it, and a modal
 * over the conversation would hide what was asked. Three answers, and refusing is one of them —
 * the model is told and answers around it.
 */
export function ToolConsent({
  request,
  onDecide,
}: {
  request: ToolRequest;
  onDecide: (requestId: string, decision: ToolDecision) => void;
}) {
  const name = useToolName();
  return (
    <div className={request.write ? "ai-ask ai-ask--write" : "ai-ask"}>
      <p className="ai-ask__head">
        <span className="ai-ask__icon">
          {request.write ? <PencilSimpleIcon weight="bold" /> : <ShieldCheckIcon weight="bold" />}
        </span>
        {request.write ? (
          <Trans>The assistant wants to change: {name(request.tool)}</Trans>
        ) : (
          <Trans>The assistant wants to read: {name(request.tool)}</Trans>
        )}
      </p>
      {/* The model's own sentence, marked as such and never on a button: what happens is the
          line above and the values below, both written here from what the host sent. */}
      {request.reason && <p className="ai-why">“{request.reason}”</p>}
      <Values params={request.params} />
      <div className="ai-ask__buttons">
        <button type="button" className="btn btn--sm" onClick={() => onDecide(request.requestId, "once")}>
          {request.write ? <Trans>Make this change</Trans> : <Trans>Allow once</Trans>}
        </button>
        {/* A change is confirmed every single time, so a card for one offers no standing
            permission: the host would refuse to honour it anyway (ADR-0037). */}
        {!request.write && (
          <>
            <button
              type="button"
              className="btn btn--sm btn--ghost"
              onClick={() => onDecide(request.requestId, "session")}
            >
              <Trans>Allow in this chat</Trans>
            </button>
            <button
              type="button"
              className="btn btn--sm btn--ghost"
              onClick={() => onDecide(request.requestId, "always")}
            >
              <Trans>Allow all</Trans>
            </button>
          </>
        )}
        <button
          type="button"
          className="btn btn--sm btn--ghost"
          onClick={() => onDecide(request.requestId, "deny")}
        >
          <Trans>Decline</Trans>
        </button>
      </div>
    </div>
  );
}

/** A run of steps as one rail. Consecutive readings are one block whatever turns they came from:
 * a dozen bordered cards is the answer buried, not the answer explained. Once the turn is over
 * and the run is long, the rail folds itself away — the count stays visible, which is the fact
 * consent rests on. */
export function Steps({ steps }: { steps: Step[] }) {
  const busy = steps.some((step) => step.busy);
  const foldable = !busy && steps.length >= 4;
  const [open, setOpen] = useState(false);
  if (steps.length === 0) return null;
  const shown = foldable && !open ? [] : steps;

  return (
    <div className="ai-steps">
      {foldable && (
        <button type="button" className="ai-steps__fold" onClick={() => setOpen((v) => !v)}>
          <DatabaseIcon />
          <span>
            <Plural value={steps.length} one="Looked at # thing" other="Looked at # things" />
          </span>
          <CaretDownIcon className={open ? "ai-caret ai-caret--on" : "ai-caret"} />
        </button>
      )}
      {shown.map((step) => (
        <StepRow key={step.key} step={step} />
      ))}
    </div>
  );
}

/**
 * One step, expandable into what went out and what came back. This is the transparency that
 * makes consent mean something: the list of calls is a fact, unlike any claim about where a
 * number in the answer came from.
 */
function StepRow({ step }: { step: Step }) {
  const { t } = useLingui();
  const name = useToolName();
  const [open, setOpen] = useState(false);
  const detail = step.sent !== undefined || step.answered !== undefined || step.text !== undefined;

  const title: ReactNode =
    step.kind === "search" ? (
      step.busy ? (
        <Trans>Searching the web: {step.name}</Trans>
      ) : (
        <Trans>Searched the web: {step.name}</Trans>
      )
    ) : step.kind === "thinking" ? (
      step.busy ? (
        <Trans>Thinking…</Trans>
      ) : (
        <Trans>How it got there</Trans>
      )
    ) : (
      <Trans>Read {name(step.name)}</Trans>
    );

  const icon =
    step.kind === "search" ? <GlobeIcon /> : step.kind === "thinking" ? <BrainIcon /> : <DatabaseIcon />;

  return (
    <div className={step.busy ? "ai-step ai-step--busy" : "ai-step"}>
      <button
        type="button"
        className="ai-step__head"
        disabled={!detail}
        aria-expanded={detail ? open : undefined}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="ai-step__icon">{step.busy ? <CircleNotchIcon className="spin" /> : icon}</span>
        <span className="ai-step__title">{title}</span>
        {detail && <CaretDownIcon className={open ? "ai-caret ai-caret--on" : "ai-caret"} />}
      </button>
      {step.why && <p className="ai-why">“{step.why}”</p>}
      {open && detail && (
        <div className="ai-step__detail">
          {step.text !== undefined && <Markdown>{step.text}</Markdown>}
          {step.sent !== undefined && (
            <>
              <p className="muted">{t`Asked for`}</p>
              <pre>{step.sent}</pre>
            </>
          )}
          {step.answered !== undefined && (
            <>
              <p className="muted">{t`Answered`}</p>
              <pre>{step.answered}</pre>
            </>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * What this chat may read from now on without asking. Shown because a permission the user
 * granted once and cannot see afterwards is not really a permission they hold.
 */
export function GrantedTools({ tools }: { tools: string[] }) {
  const name = useToolName();
  return (
    <p className="ai-granted">
      <ShieldCheckIcon />
      <span>
        <Trans>Allowed in this chat: {tools.map(name).join(", ")}</Trans>
      </span>
    </p>
  );
}

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
 * its tiers (`ai/models.rs`). The chat's current one is always an option, even when the list
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
