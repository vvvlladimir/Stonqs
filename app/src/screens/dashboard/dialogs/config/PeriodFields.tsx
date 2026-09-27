import { useLingui } from "@lingui/react/macro";
import { Field } from "../../../../components/ui";
import { useAsOf } from "../../../../lib/asOf";
import { periodLabel, usePeriodRanges } from "../../../../lib/periods";
import type { FieldsCtx } from "./context";

/** The tile's own period and the line under its value. */
export function PeriodFields({ ctx }: { ctx: FieldsCtx }) {
  const { t, i18n } = useLingui();
  const { has, set, text } = ctx;
  // The whole axis, user periods included — a widget may override the dashboard with any of them.
  const ranges = usePeriodRanges(useAsOf().date);
  return (
    <>
      {has("period") && (
        <Field
          label={t`Period`}
          hint={t`Overrides the dashboard period for this widget alone.`}
          placeholder={t`Same as the dashboard`}
          options={(ranges.data ?? []).map((r) => ({ value: r.id, label: periodLabel(i18n, r, false) }))}
          value={text("period")}
          onChange={(period) => set("period", period)}
        />
      )}

      {has("foot") && (
        <Field
          label={t`Line under the value`}
          hint={t`What the small line beneath the number says.`}
          placeholder={t`What the metric itself has to say`}
          options={[
            { value: "dates", label: t`The period's dates` },
            { value: "period", label: t`The period's name` },
            { value: "none", label: t`Nothing` },
          ]}
          value={text("foot")}
          onChange={(foot) => set("foot", foot)}
        />
      )}
    </>
  );
}
