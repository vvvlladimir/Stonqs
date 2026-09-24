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

/** Everything the app is extended with: what is installed, and what each one could not be. */
export function PluginsPanel() {
  const { t } = useLingui();
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
      info={t`A plugin adds to the app without changing what a figure means: colour themes, broker import layouts, classification sets and readers for files the app cannot open by itself.`}
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
              sub={subtitle(plugin)}
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

function subtitle(plugin: Plugin): string {
  const themes = plugin.themes.length;
  const layouts = plugin.layouts.length;
  const readers = plugin.readers.length;
  const taxonomies = plugin.taxonomies.length;
  return [
    plugin.version,
    themes > 0 ? plural(themes, { one: "# theme", other: "# themes" }) : null,
    layouts > 0 ? plural(layouts, { one: "# import layout", other: "# import layouts" }) : null,
    readers > 0 ? plural(readers, { one: "# file reader", other: "# file readers" }) : null,
    taxonomies > 0
      ? plural(taxonomies, { one: "# classification set", other: "# classification sets" })
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
