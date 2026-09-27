import { Trans, useLingui } from "@lingui/react/macro";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { usePlugins } from "../../../lib/queries";
import { PluginFrame } from "../../../components/domain/PluginFrame";
import { Empty, Pending, QueryError } from "../../../components/ui";
import type { InstalledWidget } from "../../../lib/types";
import { periodOf, pluginWidgetKey, sourceOf, type WidgetProps } from "./model";

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
  return <PluginTile key={info.key} info={info} {...props} />;
}

function PluginTile({ info, widget, date, period }: WidgetProps & { info: InstalledWidget }) {
  const ranges = usePeriodRanges(date);
  const range = info.periodic ? pickRange(ranges.data, periodOf(widget, period)) : undefined;
  // A tile reading over a period has nothing to read before the portfolio has one: said the way
  // the built-in charts say it, instead of a frame that waits for data that never comes.
  if (info.periodic && !range) {
    if (ranges.isPending) return <Pending />;
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  }
  return (
    <PluginFrame
      page={{
        kind: "widget",
        key: info.key,
        plugin: info.plugin,
        name: info.name,
        reads: info.reads,
        storage: false,
      }}
      date={date}
      range={range}
      source={sourceOf(widget)}
    />
  );
}
