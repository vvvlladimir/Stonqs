import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { useAsOf } from "../../lib/asOf";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { usePlugins } from "../../lib/queries";
import { Page } from "../../components/Page";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { PluginFrame } from "../../components/domain/PluginFrame";
import { Empty, Pending, QueryError } from "../../components/ui";

/**
 * A screen a plugin brings (ADR-0084): the app's header — its name, the plugin's, the period —
 * around the plugin's frame. It follows the app's lenses like every built-in screen.
 */
export function PluginScreen({ screenKey }: { screenKey: string | null }) {
  const { t } = useLingui();
  const asOf = useAsOf().date;
  const plugins = usePlugins();
  const [period, setPeriod] = useState<PeriodId>("YTD");
  const ranges = usePeriodRanges(asOf);
  const info = plugins.data?.screens?.find((s) => s.key === screenKey);

  if (plugins.isError) return <QueryError error={plugins.error} />;
  if (!plugins.data || (info?.periodic && ranges.isPending)) return <Pending />;
  if (!info)
    return (
      <Page archetype="plugin" title={t`Plugin screen`}>
        <Empty title={t`Plugin not installed`}>
          <Trans>The plugin that brought this screen has been removed.</Trans>
        </Empty>
      </Page>
    );

  const plugin = info.plugin_name;
  const range = info.periodic ? pickRange(ranges.data, period) : undefined;
  return (
    <Page
      archetype="plugin"
      title={info.name}
      // Whose screen this is is always said: its figures are the plugin's, not the app's (ADR-0082).
      note={t`From the ${plugin} plugin`}
      controls={
        info.periodic ? <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} /> : undefined
      }
    >
      {info.periodic && !range ? (
        <Empty title={t`No transactions yet`}>
          <Trans>This screen reads over a period, and there is nothing to read yet.</Trans>
        </Empty>
      ) : (
        <PluginFrame
          page={{
            kind: "screen",
            key: info.key,
            plugin: info.plugin,
            name: info.name,
            reads: info.reads,
            storage: info.storage,
          }}
          date={asOf}
          range={range}
        />
      )}
    </Page>
  );
}
