import { useEffect, useMemo, useRef, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../../lib/api";
import { useLanguage } from "../../../lib/i18n";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { usePluginReads, usePlugins } from "../../../lib/queries";
import {
  BRIDGE_API,
  complete,
  isFrameMessage,
  periodOfRange,
  projectPerformance,
  projectPositions,
  projectValuation,
  renderMessage,
  useThemeTokens,
  type BridgeData,
} from "../../../lib/pluginBridge";
import { Empty, ErrorText, Pending, QueryError } from "../../../components/ui";
import type { InstalledWidget } from "../../../lib/types";
import { periodOf, pluginWidgetKey, sourceOf, type WidgetProps } from "./model";

/** How long a frame may take to say it is there before the tile calls it broken. */
const START_MS = 10_000;

/** A tile drawn by a plugin (ADR-0083). The frame is the plugin's; everything around it is ours. */
export function PluginWidget(props: WidgetProps) {
  const { t } = useLingui();
  const plugins = usePlugins();
  const key = pluginWidgetKey(props.widget.type);
  const info = plugins.data?.widgets?.find((w) => w.key === key);

  if (plugins.isError) return <QueryError error={plugins.error} />;
  if (!plugins.data) return <Pending />;
  // Kept on the board rather than dropped: removing the tile is the user's decision.
  if (!info)
    return (
      <Empty title={t`Plugin not installed`}>
        <Trans>The plugin that drew this tile has been removed. Install it again, or remove the tile.</Trans>
      </Empty>
    );
  // Keyed by the widget, so a tile re-pointed at another starts a fresh frame.
  return <PluginFrame key={info.key} info={info} {...props} />;
}

interface Failure {
  code: "threw" | "no_render" | "no_start";
  detail: string;
}

function PluginFrame({ info, widget, date, period }: WidgetProps & { info: InstalledWidget }) {
  const { t } = useLingui();
  const frame = useRef<HTMLIFrameElement | null>(null);
  const [ready, setReady] = useState(false);
  const [failure, setFailure] = useState<Failure | null>(null);
  const ranges = usePeriodRanges(date);
  const range = info.periodic ? pickRange(ranges.data, periodOf(widget, period)) : undefined;
  const reads = usePluginReads(info.reads, date, range, sourceOf(widget));
  const theme = useThemeTokens();
  const { locale } = useLanguage();

  // Only this tile's own frame is listened to: every plugin frame posts to the same window.
  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      if (event.source !== frame.current?.contentWindow || !isFrameMessage(event.data)) return;
      if (event.data.type === "ready") setReady(true);
      else setFailure({ code: event.data.code, detail: event.data.detail });
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, []);

  useEffect(() => {
    if (ready) return;
    const timer = window.setTimeout(() => setFailure({ code: "no_start", detail: "" }), START_MS);
    return () => window.clearTimeout(timer);
  }, [ready]);

  const data = useMemo<BridgeData>(
    () => ({
      valuation: reads.valuation.data ? projectValuation(reads.valuation.data) : undefined,
      positions: reads.positions.data ? projectPositions(reads.positions.data) : undefined,
      performance: reads.performance.data ? projectPerformance(reads.performance.data) : undefined,
    }),
    [reads.valuation.data, reads.positions.data, reads.performance.data],
  );
  const base =
    data.valuation?.base_currency ?? data.positions?.base_currency ?? data.performance?.base_currency ?? null;

  useEffect(() => {
    const target = frame.current?.contentWindow;
    if (!ready || !target || !complete(info.reads, data)) return;
    const context = {
      api: BRIDGE_API,
      date,
      period: periodOfRange(range),
      base_currency: base,
      locale,
      theme,
    };
    // The frame has no origin to address, so `*`: what is posted is only what it declared it reads.
    target.postMessage(renderMessage(context, data), "*");
  }, [ready, info.reads, data, date, range, base, locale, theme]);

  const queryError = [reads.valuation, reads.positions, reads.performance].find((q) => q.isError)?.error;
  if (queryError) return <QueryError error={queryError} />;
  if (failure) {
    const detail = failure.detail;
    return (
      <ErrorText>
        {failure.code === "no_start"
          ? t`This plugin's tile did not start.`
          : failure.code === "no_render"
            ? t`This plugin's tile never drew anything.`
            : t`This plugin's tile failed: ${detail}`}
      </ErrorText>
    );
  }
  return (
    <iframe
      ref={frame}
      className="w__frame"
      src={api.pluginWidgetUrl(info.key)}
      // Scripts and nothing else: no origin, no storage, no popups, no forms, no navigation.
      sandbox="allow-scripts"
      title={info.name}
    />
  );
}
