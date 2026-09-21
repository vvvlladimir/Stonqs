import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { AiCustomProvider, AiWire, AppSettings } from "../../../lib/types";
import { Field, Form, Panel } from "../../../components/ui";

/**
 * A server the user points the app at: a gateway (OpenRouter, LiteLLM, Vercel AI Gateway),
 * another vendor, or a model running on this machine (Ollama, LM Studio, llama.cpp).
 *
 * Three shapes of API cover all of them, and `OPENAI_CHAT` is the one to try first: outside
 * OpenAI itself, "OpenAI-compatible" always means `POST {base}/chat/completions`. The model is
 * typed here rather than picked because a server behind a base URL need not offer a catalogue at
 * all — whatever it does offer is added to the chat's picker underneath this one.
 */
export function CustomPanel({
  settings,
  onSave,
  busy,
  error,
}: {
  settings: AppSettings;
  onSave: (custom: AiCustomProvider) => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();
  const [draft, setDraft] = useState<AiCustomProvider>(settings.ai_custom);
  const set = (patch: Partial<AiCustomProvider>) => setDraft({ ...draft, ...patch });

  const configured = settings.ai_custom.base_url.trim() !== "";
  const ready = draft.base_url.trim() !== "" && draft.model.trim() !== "";

  const WIRES: { value: AiWire; label: string }[] = [
    { value: "OPENAI_CHAT", label: t`OpenAI-compatible (chat completions)` },
    { value: "OPENAI_RESPONSES", label: t`OpenAI responses` },
    { value: "ANTHROPIC", label: t`Anthropic messages` },
    { value: "GEMINI", label: t`Gemini generateContent` },
  ];

  return (
    <Panel
      title={t`Your own provider`}
      info={t`Any server speaking one of the supported API formats: a gateway, another vendor or a model on this machine.`}
    >
      <Form
        onSubmit={() => onSave({ ...draft, label: draft.label.trim(), base_url: draft.base_url.trim() })}
        busy={busy}
        error={error}
        ready={ready}
        actions={
          configured ? (
            <button
              type="button"
              className="btn btn--sm btn--ghost"
              onClick={() => {
                const empty: AiCustomProvider = {
                  label: "",
                  base_url: "",
                  wire: "OPENAI_CHAT",
                  model: "",
                };
                setDraft(empty);
                onSave(empty);
              }}
            >
              <Trans>Remove</Trans>
            </button>
          ) : undefined
        }
      >
        <Field label={t`Name`} hint={t`What to call it in the chat's provider picker.`}>
          <input
            value={draft.label}
            placeholder={t`My provider`}
            onChange={(e) => set({ label: e.target.value })}
          />
        </Field>

        <Field
          label={t`Address`}
          hint={t`Everything up to the version, with no endpoint after it — for example https://openrouter.ai/api/v1 or http://localhost:11434/v1`}
        >
          <input
            value={draft.base_url}
            // eslint-disable-next-line lingui/no-unlocalized-strings -- a URL, not text
            placeholder="https://openrouter.ai/api/v1"
            onChange={(e) => set({ base_url: e.target.value })}
          />
        </Field>

        <Field
          label={t`API format`}
          hint={t`Start with the first one: outside OpenAI itself, "OpenAI-compatible" means exactly that shape.`}
          options={WIRES.map((wire) => ({ value: wire.value, label: wire.label }))}
          value={draft.wire}
          onChange={(wire) => set({ wire: wire as AiWire })}
        />

        <Field label={t`Model`} hint={t`The exact model id this server expects.`}>
          <input
            value={draft.model}
            placeholder="meta-llama/llama-3.3-70b-instruct"
            onChange={(e) => set({ model: e.target.value })}
          />
        </Field>
      </Form>
      <p className="muted">
        <Trans>
          Its key goes in the section above, under the name you gave it. A server on this machine usually
          needs none — leave it unset.
        </Trans>
      </p>
    </Panel>
  );
}
