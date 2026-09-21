import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { affects, useInvalidate, usePortfolio } from "../../lib/queries";
import { Async, Field, Form, Panel } from "../../components/ui";
import type { CostBasisMethod, PortfolioInput } from "../../lib/types";
import { inputOf } from "./model";

function methods(i18n: I18n): Array<[CostBasisMethod, string]> {
  return [
    ["FIFO", i18n._(msg`FIFO — the oldest lot goes first`)],
    ["AVERAGE_COST", i18n._(msg`Moving average`)],
  ];
}

/** Name, reporting currency and the rule every past sale is re-read with. */
export function PortfolioPanel() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const portfolio = usePortfolio();
  const [draft, setDraft] = useState<PortfolioInput | null>(null);

  const save = useMutation({
    mutationFn: api.portfolioSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.portfolio);
    },
  });

  return (
    <Async query={portfolio}>
      {(data) => {
        const current = draft ?? inputOf(data);
        const valid = current.name.trim() !== "" && current.base_currency.length === 3;
        return (
          <Panel title={t`Portfolio`} info={t`The portfolio's name, base currency and cost-basis method.`}>
            <Form
              onSubmit={() => save.mutate(current)}
              busy={save.isPending}
              error={save.error}
              ready={draft !== null && valid}
              onCancel={draft ? () => setDraft(null) : undefined}
            >
              <Field label={t`Portfolio name`}>
                <input
                  value={current.name}
                  onChange={(e) => setDraft({ ...current, name: e.target.value })}
                />
              </Field>

              <Field
                label={t`Base currency`}
                hint={t`The currency of all reporting. Changing it recomputes the whole history`}
              >
                <input
                  value={current.base_currency}
                  maxLength={3}
                  onChange={(e) => setDraft({ ...current, base_currency: e.target.value.toUpperCase() })}
                />
              </Field>

              <Field
                label={t`Cost-basis method`}
                hint={t`It affects the realized result of every past sale`}
                options={methods(i18n).map(([value, title]) => ({ value, label: title }))}
                value={current.cost_basis_method}
                onChange={(cost_basis_method) => setDraft({ ...current, cost_basis_method })}
              />
            </Form>
          </Panel>
        );
      }}
    </Async>
  );
}
