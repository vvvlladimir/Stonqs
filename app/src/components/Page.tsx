/* eslint-disable lingui/no-unlocalized-strings -- the archetype rules below are developer
   diagnostics printed to the console in dev builds; they never reach a user. */
import { Trans } from "@lingui/react/macro";
import type { ReactNode } from "react";
import { ArrowLeftIcon } from "@phosphor-icons/react";

/** Page archetypes define allowed slots and validate them in development. */

export type Archetype = "overview" | "registry" | "analysis" | "object" | "wizard" | "form";

type Slot = "head" | "lead" | "steps" | "controls" | "filters" | "banner" | "metrics" | "body" | "foot";

interface Rule {
  allow: Slot[];
  require: Slot[];
  deny?: Partial<Record<Slot, string>>;
}

const ARCHETYPES: Record<Archetype, Rule> = {
  /* Overview is widget-driven, so it has no title or shared metric slot. */
  overview: {
    allow: ["controls", "banner", "body"],
    require: ["controls", "body"],
    deny: { metrics: "the overview total lives in widgets, not on a shared tile shelf" },
  },
  /* Registry uses a one-line summary; it does not provide analysis metrics. A registry may
     carry controls once a column of it is computed over a period rather than read off a date —
     the positions table showing TWR is the case. Its total still belongs in the summary line. */
  registry: {
    allow: ["head", "controls", "filters", "banner", "body"],
    require: ["head", "body"],
    deny: { metrics: "a registry shows its total in the summary line, not as tiles" },
  },
  /* Analysis requires controls so the period or window is visible. */
  analysis: {
    allow: ["head", "lead", "controls", "filters", "banner", "metrics", "body"],
    require: ["head", "controls", "metrics", "body"],
  },
  /* Object views inherit their period from the source screen. */
  object: {
    allow: ["head", "banner", "body"],
    require: ["head", "body"],
    deny: {
      controls: "an object has no axis of its own: the period comes from the screen it was opened from",
    },
  },
  /* Wizard is the only linear archetype with a footer. */
  wizard: {
    allow: ["head", "lead", "steps", "banner", "body", "foot"],
    require: ["head", "steps", "body", "foot"],
    deny: { metrics: "a wizard computes nothing; it assembles a decision" },
  },
  /* Forms edit settings; they have neither metrics nor an analysis axis. */
  form: {
    allow: ["head", "lead", "body"],
    require: ["head", "body"],
    deny: { metrics: "a form has nothing to measure", controls: "a form has no axis" },
  },
};

export interface PageProps {
  archetype: Archetype;
  /** Header slot; a title always renders inside the header row. */
  title?: ReactNode;
  /** Back button allowed only for object pages. */
  back?: { label: string; onClick: () => void };
  /** One-line registry summary. */
  summary?: ReactNode;
  /** Valuation date shown by registry and as-of analysis pages. */
  asOf?: string;
  note?: ReactNode;
  actions?: ReactNode;
  lead?: ReactNode;
  steps?: ReactNode;
  controls?: ReactNode;
  filters?: ReactNode;
  banner?: ReactNode;
  metrics?: ReactNode;
  foot?: ReactNode;
  children: ReactNode;
}

function violationsOf(props: PageProps): string[] {
  const rule = ARCHETYPES[props.archetype];
  const present: Slot[] = [];
  if (props.title) present.push("head");
  if (props.lead) present.push("lead");
  if (props.steps) present.push("steps");
  if (props.controls) present.push("controls");
  if (props.filters) present.push("filters");
  if (props.banner) present.push("banner");
  if (props.metrics) present.push("metrics");
  if (props.children) present.push("body");
  if (props.foot) present.push("foot");

  const out: string[] = [];
  for (const slot of present) {
    const denied = rule.deny?.[slot];
    if (denied) out.push(`${props.archetype}: slot "${slot}" is forbidden — ${denied}`);
    else if (!rule.allow.includes(slot))
      out.push(`${props.archetype}: slot "${slot}" is not part of the archetype`);
  }
  for (const slot of rule.require) {
    if (!present.includes(slot)) out.push(`${props.archetype}: required slot "${slot}" is missing`);
  }
  return out;
}

export function Page(props: PageProps) {
  const { archetype, title, back, summary, asOf, note, actions, lead, steps } = props;
  const { controls, filters, banner, metrics, foot, children } = props;

  // Validate only in development; production builds pay no runtime cost.
  const violations = import.meta.env.DEV ? violationsOf(props) : [];

  const meta = summary || asOf || note;

  return (
    <div className="page" data-archetype={archetype}>
      {violations.length > 0 && (
        <div className="violations" role="alert">
          {violations.map((v) => (
            <span key={v}>{v}</span>
          ))}
        </div>
      )}

      {title && (
        <div className="page__head" data-slot="head">
          <div className="page__title">
            {back && (
              <button type="button" className="iconbtn page__back" onClick={back.onClick}>
                <ArrowLeftIcon /> {back.label}
              </button>
            )}
            <h1>{title}</h1>
            {meta && (
              <div className="inline">
                {summary && <span className="page__summary num">{summary}</span>}
                {asOf && (
                  <span className="page__meta">
                    <Trans>as of {asOf}</Trans>
                  </span>
                )}
                {note && <span className="page__meta">{note}</span>}
              </div>
            )}
          </div>
          {/* Keep analysis controls in the header to preserve vertical space. */}
          {controls && (
            <div className="page__controls" data-slot="controls">
              {controls}
            </div>
          )}
          {actions && <div className="page__actions">{actions}</div>}
        </div>
      )}

      {lead && (
        <p className="page__lead" data-slot="lead">
          {lead}
        </p>
      )}
      {steps && (
        <div className="steps" data-slot="steps">
          {steps}
        </div>
      )}
      {/* Without a title, controls occupy their own row. */}
      {controls && !title && (
        <div className="page__controls" data-slot="controls">
          {controls}
        </div>
      )}
      {filters && (
        <div className="page__filters" data-slot="filters">
          {filters}
        </div>
      )}
      {banner && <div data-slot="banner">{banner}</div>}
      {metrics}
      <div className="page__body" data-slot="body">
        {children}
      </div>
      {foot && (
        <div className="page__foot" data-slot="foot">
          {foot}
        </div>
      )}
    </div>
  );
}
