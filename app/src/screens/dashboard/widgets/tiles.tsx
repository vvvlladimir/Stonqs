import { Trans, useLingui } from "@lingui/react/macro";
import { METRICS } from "./metrics";
import { periodOf, sourceOf, type WidgetProps } from "./model";

/** The metric tile: renders whichever catalog entry the widget settings name. */
export function MetricWidget({ widget, date, period }: WidgetProps) {
  const def = METRICS[String(widget.cfg.metric)];
  if (!def)
    return (
      <p className="muted">
        <Trans>Metric not found.</Trans>
      </p>
    );
  return (
    <>
      {def.Render({
        date,
        period: periodOf(widget, period),
        source: sourceOf(widget),
        foot: typeof widget.cfg.foot === "string" ? widget.cfg.foot : "",
      })}
    </>
  );
}

/** Borderless section divider: text and nothing else. */
export function HeadingWidget({ widget }: WidgetProps) {
  const { t } = useLingui();
  const own = typeof widget.cfg.title === "string" ? widget.cfg.title.trim() : "";
  const text = own || t`Section`;
  return <div className="group-label">{text}</div>;
}
