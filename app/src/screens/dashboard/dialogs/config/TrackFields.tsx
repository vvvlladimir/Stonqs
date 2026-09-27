import { useLingui } from "@lingui/react/macro";
import { Field } from "../../../../components/ui";
import { TRACK_LABELS, trackOf, type Track } from "../../widgets/model";
import type { FieldsCtx } from "./context";

/** Which figure a progress tile tracks, and the FIRE assumptions when that is the one. */
export function TrackFields({ ctx }: { ctx: FieldsCtx }) {
  const { t, i18n } = useLingui();
  const { draft, has, tracks, set, text } = ctx;
  return (
    <>
      {has("track") && (
        <Field
          label={t`What it tracks`}
          hint={t`All three read as one figure over a track; this picks which figure.`}
          options={(Object.keys(TRACK_LABELS) as Track[]).map((track) => ({
            value: track,
            label: i18n._(TRACK_LABELS[track]),
          }))}
          value={trackOf(draft.cfg)}
          onChange={(track) => set("track", track)}
        />
      )}

      {tracks("fire") && (
        <>
          <Field
            label={t`Spending to cover, a year`}
            hint={t`In the reporting currency. Everything else on this tile follows from it.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("spending")}
              placeholder={t`24000`}
              onChange={(e) => set("spending", e.target.value)}
            />
          </Field>
          <Field
            label={t`Withdrawal rate`}
            hint={t`The share of the capital taken out each year. 0.04 is the common rule of thumb and means the target is twenty-five years of spending.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("withdrawal")}
              placeholder="0.04"
              onChange={(e) => set("withdrawal", e.target.value)}
            />
          </Field>
          <Field
            label={t`Expected return, a year`}
            hint={t`An assumption, never this portfolio's measured return. Subtract inflation yourself to read the answer in today's money.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("return")}
              placeholder="0.05"
              onChange={(e) => set("return", e.target.value)}
            />
          </Field>
          <Field
            label={t`Paid in a month`}
            hint={t`Left empty, the tile uses what the active plans already add up to in a month.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("contribution")}
              placeholder={t`What the plans contribute`}
              onChange={(e) => set("contribution", e.target.value)}
            />
          </Field>
        </>
      )}
    </>
  );
}
