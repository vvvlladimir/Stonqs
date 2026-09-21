import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { affects, useInvalidate } from "../../lib/queries";
import { Field, FormDialog } from "../../components/ui";
import type { AlertRow, DevAlertStep } from "../../lib/types";

/**
 * Debug builds only: moves the price the way a market would, so the log, the navigation dot and
 * the OS notification can be watched without waiting for one. "Cross" writes today's close one
 * percent past the level; "Reset" takes the simulated closes back, which crosses again.
 */
export function DevSimulator({ rows, onClose }: { rows: AlertRow[]; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [alertId, setAlertId] = useState(rows[0]?.alert.id ?? "");
  const [step, setStep] = useState<DevAlertStep>("cross");

  const simulate = useMutation({
    mutationFn: () => api.devAlertSimulate(alertId, step),
    onSuccess: () => invalidate(...affects.quotes),
  });

  return (
    <FormDialog
      title={t`Simulate a price move`}
      onClose={onClose}
      onSubmit={() => simulate.mutate()}
      busy={simulate.isPending}
      error={simulate.error}
      ready={alertId !== ""}
      submitLabel={t`Apply`}
      busyLabel={t`Applying…`}
    >
      <Field
        label={t`Alert`}
        options={rows.map((row) => ({
          value: row.alert.id,
          label: `${row.symbol} · ${row.alert.price ?? row.alert.date ?? ""} ${row.alert.currency ?? ""}`,
        }))}
        value={alertId}
        onChange={setAlertId}
      />
      <Field
        label={t`Move`}
        hint={
          simulate.data === undefined
            ? t`The next check runs right away and notifies like a real refresh.`
            : t`Crossings logged: ${simulate.data}`
        }
        options={[
          { value: "cross", label: t`Cross the level (or reach the date today)` },
          { value: "reset", label: t`Take the simulated closes back` },
        ]}
        value={step}
        onChange={setStep}
      />
    </FormDialog>
  );
}
