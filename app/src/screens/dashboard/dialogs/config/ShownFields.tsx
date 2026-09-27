import type { I18n } from "@lingui/core";
import { useLingui } from "@lingui/react/macro";
import { Field } from "../../../../components/ui";
import { scopeLabel } from "../../../../components/domain/scopeLabel";
import { useScope } from "../../../../lib/queries";
import { METRICS } from "../../widgets/metrics";
import { RATIO_TERMS, ratioTerms, sourceFromKey, sourceKey, sourceOf } from "../../widgets/model";
import type { FieldsCtx } from "./context";

/** Title, data source and what figure the tile shows. */
export function ShownFields({ ctx }: { ctx: FieldsCtx }) {
  const { t, i18n } = useLingui();
  const { def, draft, has, set, text } = ctx;
  const scope = useScope();
  return (
    <>
      {has("title") && (
        <Field label={t`Title`}>
          <input
            type="text"
            value={text("title")}
            placeholder={def.titleOf ? def.titleOf(i18n, draft.cfg) : i18n._(def.label)}
            onChange={(e) => set("title", e.target.value)}
          />
        </Field>
      )}

      {has("source") && (
        <Field
          label={t`Data source`}
          hint={t`Left empty, the widget follows the picker like every screen does; naming one pins this tile to it.`}
          placeholder={t`Follow the picker`}
          options={(scope.data?.options ?? []).map((option) => ({
            value: sourceKey(option),
            label: scopeLabel(i18n, option),
          }))}
          value={sourceKey(sourceOf(draft))}
          onChange={(value) => set("source", sourceFromKey(value) ?? null)}
        />
      )}

      {has("metric") && (
        <Field
          label={t`Metric`}
          hint={metricTip(i18n, text("metric"))}
          options={Object.entries(METRICS).map(([key, metric]) => ({
            value: key,
            label: i18n._(metric.label),
          }))}
          value={text("metric")}
          onChange={(metric) => set("metric", metric)}
        />
      )}

      {has("ratio") && (
        <>
          <Field
            label={t`Top`}
            options={termOptions(i18n)}
            value={ratioTerms(draft.cfg).top}
            onChange={(top) => set("top", top)}
          />
          <Field
            label={t`Divided by`}
            options={termOptions(i18n)}
            value={ratioTerms(draft.cfg).bottom}
            onChange={(bottom) => set("bottom", bottom)}
          />
          <Field
            label={t`Shown as`}
            options={[
              { value: "percent", label: t`A percentage` },
              { value: "multiple", label: t`A multiple, like 2.5×` },
            ]}
            value={draft.cfg.as === "multiple" ? "multiple" : "percent"}
            onChange={(as) => set("as", as)}
          />
        </>
      )}
    </>
  );
}

/** Both sides of a ratio are picked from the same list; either may be a period figure. */
function termOptions(i18n: I18n) {
  return Object.entries(RATIO_TERMS).map(([key, term]) => ({ value: key, label: i18n._(term.label) }));
}

/** The metric tip, resolved only when the chosen metric exists. */
function metricTip(i18n: I18n, key: string): string | undefined {
  const metric = METRICS[key];
  return metric ? i18n._(metric.tip) : undefined;
}
