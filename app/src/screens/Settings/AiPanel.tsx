import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { useProviderName } from "../../lib/ai";
import { keys, useAiProviders, useInvalidate, useSettings } from "../../lib/queries";
import { CUSTOM_PROVIDER } from "../../lib/kinds";
import { CheckField, Field, Form, Panel, Pending, QueryError } from "../../components/ui";
import type { AiCustomProvider } from "../../lib/types";
import { CustomPanel } from "./ai/CustomPanel";
import { KeysPanel } from "./ai/KeysPanel";
import { UsagePanel } from "./ai/UsagePanel";

/**
 * BYOK only: the key is saved straight to the OS keychain and never comes back to this screen.
 *
 * There is no model here. A chat lands on whatever its provider offers first today, and the one
 * place a model id is written down is the custom server, where the user supplies both the
 * address and what to ask it for.
 */
export function AiPanel() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const settings = useSettings();
  const providers = useAiProviders();
  const name = useProviderName();

  const save = useMutation({
    mutationFn: api.settingsSave,
    // The model list is the custom server's own, and this screen can change which server that
    // is; the built-in providers' lists are untouched by anything here.
    onSuccess: () => invalidate(keys.settings(), keys.aiProviders(), keys.aiModels(CUSTOM_PROVIDER)),
  });

  if (settings.isError) return <QueryError error={settings.error} />;
  if (!settings.data) return <Pending />;
  const current = settings.data;
  const listed = providers.data ?? [];

  const saveCustom = (ai_custom: AiCustomProvider) => {
    // Removing the server takes the chats' provider with it only at the next send: a chat
    // already on it keeps saying so, and the picker tells the user it is no longer configured.
    save.mutate({ ...current, ai_custom });
  };

  return (
    <>
      <Panel
        title={t`AI assistant`}
        info={t`A chat that reads your portfolio and answers through an AI provider you connect.`}
      >
        <Form>
          <CheckField
            label={t`Enable the AI panel`}
            checked={current.ai_enabled}
            onChange={(ai_enabled) => save.mutate({ ...current, ai_enabled })}
          />

          <Field
            label={t`New chats start on`}
            hint={t`A chat keeps its own provider and model afterwards, switchable from inside it.`}
            options={listed.map((provider) => ({
              value: provider.id,
              label: provider.connected
                ? name(provider.id, provider.label)
                : t`${name(provider.id, provider.label)} — no key saved`,
            }))}
            value={current.ai_provider}
            onChange={(ai_provider) => save.mutate({ ...current, ai_provider })}
          />

          <CheckField
            label={t`Let the assistant search the web`}
            hint={t`Your question is sent to the provider's search when it looks something up. Only the built-in providers do this.`}
            checked={current.ai_web_search}
            onChange={(ai_web_search) => save.mutate({ ...current, ai_web_search })}
          />

          <CheckField
            label={t`Show how the assistant got there`}
            hint={t`A short summary of its own reasoning, folded away above each answer. It costs a few extra tokens, and some provider accounts are not cleared to produce one.`}
            checked={current.ai_reasoning}
            onChange={(ai_reasoning) => save.mutate({ ...current, ai_reasoning })}
          />
        </Form>
      </Panel>

      <KeysPanel providers={listed} />
      <CustomPanel
        key={JSON.stringify(current.ai_custom)}
        settings={current}
        busy={save.isPending}
        error={save.error}
        onSave={saveCustom}
      />
      <UsagePanel />
    </>
  );
}
