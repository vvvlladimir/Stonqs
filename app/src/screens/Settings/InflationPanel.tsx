import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { affects, useInflationStatus, useInvalidate } from "../../lib/queries";
import { Async, Field, Form, Panel } from "../../components/ui";
import { formatDay, formatRegion } from "../../lib/format";

/**
 * Where the portfolio's owner spends, which is what real returns are measured against.
 *
 * The host sends region codes only, so the names are written here — `Intl` has every country in
 * the languages the app ships, and the two aggregates it has no code for are translated.
 */
export function InflationPanel() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const status = useInflationStatus();
  const [draft, setDraft] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: (region: string) => api.inflationRegionSet(region === "" ? null : region),
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.portfolio);
    },
  });

  return (
    <Async query={status}>
      {(data) => {
        const current = draft ?? data.region ?? "";
        const options = data.regions
          .map((code) => ({ value: code, label: formatRegion(code) }))
          .sort((a, b) => a.label.localeCompare(b.label));
        return (
          <Panel
            title={t`Inflation`}
            info={t`Real returns are measured against the prices of one region, not against the base currency.`}
            note={
              data.published_through ? (
                <Trans>published through {formatDay(data.published_through)}</Trans>
              ) : undefined
            }
          >
            <Form
              onSubmit={() => save.mutate(current)}
              busy={save.isPending}
              error={save.error}
              ready={draft !== null}
              onCancel={draft !== null ? () => setDraft(null) : undefined}
            >
              <Field
                label={t`Price-index region`}
                hint={t`Where you spend, which need not be where your base currency is legal tender. Left empty, no real returns are shown.`}
                placeholder={t`Do not report real returns`}
                options={options}
                value={current}
                onChange={setDraft}
              />
            </Form>
          </Panel>
        );
      }}
    </Async>
  );
}
