import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open as pickFolder } from "@tauri-apps/plugin-dialog";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { keys, useInvalidate, usePlugins } from "../../lib/queries";
import { useErrorText } from "../../components/ui/Async";
import { Banner, Empty, ErrorText, List, ListRow, Panel, Pending, QueryError } from "../../components/ui";
import type { Plugin } from "../../lib/types";
import { WIDGET_READ_LABELS } from "../../lib/kinds";

/** Everything the app is extended with: what is installed, and what each one could not be. */
export function PluginsPanel() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const plugins = usePlugins();
  const [failure, setFailure] = useState<unknown>(null);

  const install = useMutation({
    mutationFn: async () => {
      const folder = await pickFolder({ directory: true, title: t`Choose a plugin folder` });
      if (typeof folder !== "string") return;
      await api.pluginInstall(folder);
    },
    onSuccess: () => invalidate(keys.plugins()),
    onError: setFailure,
  });

  const remove = useMutation({
    mutationFn: api.pluginRemove,
    onSuccess: () => invalidate(keys.plugins(), keys.pluginTheme()),
    onError: setFailure,
  });

  if (plugins.isError) return <QueryError error={plugins.error} />;
  if (!plugins.data) return <Pending />;
  const list = plugins.data.plugins;

  return (
    <Panel
      title={t`Plugins`}
      info={t`A plugin adds to the app without changing what a figure means: colour themes, broker import layouts, classification sets, operation words in another language, readers for files the app cannot open by itself, formats to export to and dashboard widgets.`}
      tools={
        <button
          className="btn btn--ghost btn--sm"
          disabled={install.isPending}
          onClick={() => install.mutate()}
        >
          <PlusIcon /> <Trans>Install from folder…</Trans>
        </button>
      }
    >
      {failure !== null && <InstallError error={failure} />}

      {list.length === 0 ? (
        <Empty title={t`Nothing installed`}>
          <Trans>A plugin is a folder holding a plugin.json and the files it names.</Trans>
        </Empty>
      ) : (
        <List>
          {list.map((plugin) => (
            <ListRow
              key={plugin.id}
              title={plugin.name}
              sub={subtitle(plugin, i18n)}
              foot={plugin.status === "ok" ? undefined : <Banner tone="warn">{reason(plugin, t)}</Banner>}
              showActions
              end={
                <button
                  className="btn btn--ghost btn--sm"
                  disabled={remove.isPending}
                  onClick={() => remove.mutate(plugin.id)}
                >
                  <TrashIcon /> <Trans>Remove</Trans>
                </button>
              }
            />
          ))}
        </List>
      )}
    </Panel>
  );
}

function InstallError({ error }: { error: unknown }) {
  return <ErrorText>{useErrorText(error)}</ErrorText>;
}

type T = ReturnType<typeof useLingui>["t"];
type I18n = ReturnType<typeof useLingui>["i18n"];

function subtitle(plugin: Plugin, i18n: I18n): string {
  const themes = plugin.themes.length;
  const layouts = plugin.layouts.length;
  const readers = plugin.readers.length;
  const taxonomies = plugin.taxonomies.length;
  const dictionaries = plugin.dictionaries.length;
  const writers = plugin.writers.length;
  const widgets = plugin.widgets?.length ?? 0;
  // What a widget is handed is said here as well as in the palette: installing is when the user
  // first sees the package, placing a tile is when it first reads (ADR-0083).
  const reads = [...new Set((plugin.widgets ?? []).flatMap((w) => w.reads))]
    .map((read) => i18n._(WIDGET_READ_LABELS[read]))
    .join(", ");
  return [
    plugin.version,
    themes > 0 ? plural(themes, { one: "# theme", other: "# themes" }) : null,
    layouts > 0 ? plural(layouts, { one: "# import layout", other: "# import layouts" }) : null,
    readers > 0 ? plural(readers, { one: "# file reader", other: "# file readers" }) : null,
    writers > 0 ? plural(writers, { one: "# export format", other: "# export formats" }) : null,
    widgets > 0
      ? reads
        ? plural(widgets, {
            one: `# dashboard widget, given ${reads}`,
            other: `# dashboard widgets, given ${reads}`,
          })
        : plural(widgets, { one: "# dashboard widget", other: "# dashboard widgets" })
      : null,
    taxonomies > 0
      ? plural(taxonomies, { one: "# classification set", other: "# classification sets" })
      : null,
    dictionaries > 0
      ? plural(dictionaries, { one: "# operation dictionary", other: "# operation dictionaries" })
      : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

/** Why a plugin is not in use, said in the user's language from the host's code. */
function reason(plugin: Plugin, t: T): string {
  switch (plugin.status) {
    case "ok":
      return "";
    case "api":
      return t`Built for plugin version ${plugin.wants}; this app speaks ${plugin.speaks}. Update the app, or the plugin.`;
    case "broken":
      return t`Its plugin.json could not be read: ${plugin.detail}`;
  }
}
