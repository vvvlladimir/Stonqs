import { useState, type ReactNode } from "react";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import {
  BrainIcon,
  CaretDownIcon,
  CircleNotchIcon,
  DatabaseIcon,
  GlobeIcon,
  PencilSimpleIcon,
  ShieldCheckIcon,
} from "@phosphor-icons/react";
import type { ToolDecision, ToolParams } from "../../lib/types";
import type { ToolRequest } from "../../lib/ai";
import { Markdown } from "../ui/Markdown";
import {
  PARAM_LABELS,
  PLUGIN_TOOL_LABEL,
  PLUGIN_TOOL_PREFIX,
  TOOL_LABELS,
  paramEntries,
} from "./aiToolLabels";
import type { Step } from "./aiSteps";

function useToolName() {
  const { i18n } = useLingui();
  return (tool: string) =>
    TOOL_LABELS[tool]
      ? i18n._(TOOL_LABELS[tool])
      : tool.startsWith(PLUGIN_TOOL_PREFIX)
        ? i18n._(PLUGIN_TOOL_LABEL)
        : tool;
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
