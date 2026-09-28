import { msg } from "@lingui/core/macro";
import { SparkleIcon, TextHOneIcon } from "@phosphor-icons/react";
import { BriefWidget } from "../ai";
import { HeadingWidget } from "../tiles";
import type { WidgetDef } from "../model";

/** Widgets that are words rather than figures. */
export const TEXT: Record<string, WidgetDef> = {
  brief: {
    label: msg`Portfolio summary`,
    icon: SparkleIcon,
    description: msg`What the period did and what drove it, written by the assistant on request.`,
    group: msg`Text`,
    size: { w: 6, h: 6 },
    min: { w: 3, h: 4 },
    fields: ["title", "source", "period", "prompt", "length", "refresh", "model"],
    Render: BriefWidget,
  },
  heading: {
    label: msg`Section heading`,
    icon: TextHOneIcon,
    description: msg`A divider line: it splits the dashboard into meaningful parts.`,
    group: msg`Text`,
    size: { w: 12, h: 2 },
    min: { w: 12, h: 2 },
    fields: ["title"],
    plain: true,
    Render: HeadingWidget,
  },
};
