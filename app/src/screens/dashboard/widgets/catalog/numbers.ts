import { msg } from "@lingui/core/macro";
import { DivideIcon, FlagCheckeredIcon, GaugeIcon } from "@phosphor-icons/react";
import { METRICS } from "../metrics";
import { RATIO_TERMS, TRACK_LABELS, ratioTerms, trackOf } from "../model";
import { ProgressWidget } from "../progress";
import { RatioWidget } from "../ratio";
import { MetricWidget } from "../tiles";
import type { WidgetDef } from "../model";

/** Widgets that show one figure: a metric, a ratio, a figure over a track. */
export const NUMBERS: Record<string, WidgetDef> = {
  metric: {
    label: msg`Metric`,
    icon: GaugeIcon,
    description: msg`One number from the catalog: value, return, drawdown.`,
    group: msg`Numbers`,
    size: { w: 3, h: 4 },
    min: { w: 2, h: 3 },
    fields: ["title", "source", "metric", "period", "foot"],
    defaults: { metric: "value" },
    titleOf: (i18n, cfg) => {
      const metric = METRICS[String(cfg.metric)];
      return metric ? i18n._(metric.label) : i18n._(msg`Metric`);
    },
    periodicFor: (cfg) => METRICS[String(cfg.metric)]?.periodic === true,
    Render: MetricWidget,
  },
  ratio: {
    label: msg`Ratio`,
    icon: DivideIcon,
    description: msg`One figure over another: a yield, a cost share, how much of it is cash.`,
    group: msg`Numbers`,
    size: { w: 3, h: 4 },
    min: { w: 2, h: 3 },
    fields: ["title", "source", "ratio", "period", "foot"],
    defaults: { top: "income", bottom: "value" },
    titleOf: (i18n, cfg) => {
      const { top, bottom } = ratioTerms(cfg);
      return `${i18n._(RATIO_TERMS[top].label)} / ${i18n._(RATIO_TERMS[bottom].label)}`;
    },
    periodicFor: (cfg) => {
      const { top, bottom } = ratioTerms(cfg);
      return RATIO_TERMS[top].periodic === true || RATIO_TERMS[bottom].periodic === true;
    },
    Render: RatioWidget,
  },
  progress: {
    label: msg`Progress`,
    icon: FlagCheckeredIcon,
    description: msg`A figure over a track: a savings goal, a contribution limit, or financial independence.`,
    group: msg`Numbers`,
    size: { w: 4, h: 5 },
    min: { w: 2, h: 4 },
    fields: ["title", "track", "goal", "limit", "fire"],
    defaults: { track: "goal", withdrawal: "0.04", return: "0.05" },
    titleOf: (i18n, cfg) => i18n._(TRACK_LABELS[trackOf(cfg)]),
    Render: ProgressWidget,
  },
};
