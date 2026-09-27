import { useLingui } from "@lingui/react/macro";
import { Field } from "../../../../components/ui";
import { useProviderName } from "../../../../lib/ai";
import { useAiModels, useAiProviders, useSettings } from "../../../../lib/queries";
import type { Widget } from "../../../../lib/uiState";
import { lengthOf, modelOf } from "../../widgets/model";
import type { FieldsCtx } from "./context";

/** The summary tile: what it is asked, how long, how often, and by which model. */
export function BriefFields({ ctx }: { ctx: FieldsCtx }) {
  const { t } = useLingui();
  const { draft, setDraft, has, set, text } = ctx;
  return (
    <>
      {has("prompt") && (
        <Field
          label={t`What this tile should say`}
          hint={t`Left empty, it writes what the period did and what drove it. Whatever you ask for, it still states only figures it was given.`}
        >
          <textarea
            rows={3}
            value={text("prompt")}
            placeholder={t`What moved the portfolio this period`}
            onChange={(e) => set("prompt", e.target.value)}
          />
        </Field>
      )}

      {has("length") && (
        <Field
          label={t`Answer length, tokens`}
          hint={t`A token is about three quarters of an English word, less in Russian. Left empty, the model decides how long to be.`}
        >
          <input
            type="number"
            min={50}
            max={4000}
            step={50}
            value={lengthOf(draft.cfg) ?? ""}
            placeholder={t`No limit`}
            // Only the ceiling is clamped while typing — a floor would turn the "3" of "300" into
            // 50. The host raises anything too small to be an answer.
            onChange={(e) => {
              const tokens = Math.round(Number(e.target.value));
              set("max_tokens", tokens > 0 ? Math.min(tokens, 4000) : null);
            }}
          />
        </Field>
      )}

      {has("refresh") && (
        <Field
          label={t`Write again on its own`}
          hint={t`Each rewrite is a paid request to the model, so this is off unless you ask for it. Off, the tile says when it is out of date and waits.`}
          placeholder={t`Only when I ask`}
          options={[
            { value: "daily", label: t`Once a day` },
            { value: "weekly", label: t`Once a week` },
            { value: "monthly", label: t`Once a month` },
          ]}
          value={text("refresh")}
          onChange={(refresh) => set("refresh", refresh)}
        />
      )}

      {has("model") && (
        <ModelFields
          widget={draft}
          onChange={(provider, model) => setDraft({ ...draft, cfg: { ...draft.cfg, provider, model } })}
        />
      )}
    </>
  );
}

/** Its own component so only a tile offering it fetches the model list; switching provider clears the model. */
function ModelFields({
  widget,
  onChange,
}: {
  widget: Widget;
  onChange: (provider: string | null, model: string | null) => void;
}) {
  const { t } = useLingui();
  const name = useProviderName();
  const settings = useSettings();
  const providers = useAiProviders();
  const chosen = modelOf(widget);
  const listed = chosen.provider ?? settings.data?.ai_provider ?? null;
  const models = useAiModels(listed, true);
  // A keyless provider is shown but disabled; the chosen one always stays pickable.
  const listedProviders = providers.data ?? [];
  const modelOptions = [...(models.data ?? [])];
  if (chosen.model && !modelOptions.includes(chosen.model)) modelOptions.unshift(chosen.model);

  return (
    <>
      <Field
        label={t`Provider`}
        hint={t`Left empty, the tile is written where a new chat begins.`}
        placeholder={t`Same as a new chat`}
        options={listedProviders.map((p) => {
          const usable = p.connected || p.id === chosen.provider;
          return {
            value: p.id,
            label: usable ? name(p.id, p.label) : t`${name(p.id, p.label)} — no key saved`,
            disabled: !usable,
          };
        })}
        value={chosen.provider ?? ""}
        onChange={(provider) => onChange(provider || null, null)}
      />
      <Field
        label={t`Model`}
        hint={t`Left empty, the newest model the provider offers.`}
        placeholder={models.isPending ? t`Loading the model list…` : t`The newest`}
        options={modelOptions.map((m) => ({ value: m, label: m }))}
        value={chosen.model ?? ""}
        // A model is stored with the provider it came from, so the default provider is written
        // down the moment a model of its is picked.
        onChange={(model) => onChange(model ? listed : chosen.provider, model || null)}
      />
    </>
  );
}
