import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { keys, useInvalidate, usePlugins, useSettings } from "../../lib/queries";
import { LOCALE_NAMES, PREFERENCES, useLanguage } from "../../lib/i18n";
import { useUiState } from "../../lib/uiState";
import { pluginTheme, themeOfPlugin, type ThemePreference } from "../../lib/theme";
import { Field, Form, Panel, Pending, QueryError, Seg } from "../../components/ui";

/** Language and colour scheme: the language is host settings, the scheme is UI state. */
export function AppearancePanel() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const settings = useSettings();
  const { preference } = useLanguage();
  const { ui, save: saveUi } = useUiState();
  const plugins = usePlugins();

  const save = useMutation({
    mutationFn: api.settingsSave,
    onSuccess: () => invalidate(keys.settings()),
  });

  if (settings.isError) return <QueryError error={settings.error} />;
  if (!settings.data) return <Pending />;
  const current = settings.data;
  const installed = plugins.data?.themes ?? [];

  const themes: Array<{ value: ThemePreference; label: string }> = [
    { value: "system", label: t`System` },
    { value: "light", label: t`Light` },
    { value: "dark", label: t`Dark` },
  ];

  return (
    <Panel title={t`Appearance`} info={t`The interface language and the colour scheme.`}>
      <Form>
        <Field
          label={t`Interface language`}
          hint={t`"System" follows the language your operating system reports. Dates and numbers follow it too.`}
          options={PREFERENCES.map((value) => ({
            value,
            label: value === "system" ? t`System` : LOCALE_NAMES[value],
          }))}
          value={preference}
          onChange={(language) => save.mutate({ ...current, language })}
        />

        <Field label={t`Colour scheme`} hint={t`"System" follows your operating system, light or dark.`}>
          <Seg
            label={t`Colour scheme`}
            options={themes}
            value={pluginTheme(ui.theme) ? "system" : ui.theme}
            onChange={(theme) => saveUi({ ...ui, theme })}
          />
        </Field>

        {installed.length > 0 && (
          <Field
            label={t`Installed theme`}
            hint={t`A theme from a plugin. It varies one of the two schemes above rather than replacing it.`}
            options={[
              { value: "", label: t`None` },
              ...installed.map((theme) => ({ value: themeOfPlugin(theme.key), label: theme.name })),
            ]}
            value={pluginTheme(ui.theme) ? ui.theme : ""}
            onChange={(value) =>
              saveUi({ ...ui, theme: value === "" ? "system" : (value as ThemePreference) })
            }
          />
        )}
      </Form>
    </Panel>
  );
}
