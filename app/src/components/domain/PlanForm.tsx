import { Trans, useLingui } from "@lingui/react/macro";
import { PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { CheckField, Field, FieldRow, FieldSet, FormDialog, type SelectOption } from "../../components/ui";
import { formatPercent } from "../../lib/format";
import { PLAN_CADENCES } from "../../lib/kinds";
import { amountOf, legShare } from "../../lib/plans";
import type { AccountRow, PlanInput, SecurityRow } from "../../lib/types";

/**
 * The editor of one contribution plan, shared by the Plans screen and by Rebalance, which
 * opens it prefilled from the trades it proposed.
 *
 * A plan with no instruments is a cash contribution and is a legitimate plan, so the form
 * never demands a security before it will save. The weights are shown resolved as percentages
 * while they are typed: "1 1 1" and "60 40" are both valid and both need reading back.
 */
export function PlanForm({
  draft,
  accounts,
  securities,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
  title,
  submitLabel,
}: {
  draft: PlanInput;
  accounts: AccountRow[];
  securities: SecurityRow[];
  onChange: (draft: PlanInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
  /** Overridden where the dialog is opened from somewhere other than the plan list. */
  title?: string;
  submitLabel?: string;
}) {
  const { t, i18n } = useLingui();

  // A leg buys instruments, so it needs a securities account; a bare contribution lands on cash.
  const depots = accounts.filter((a) => a.kind === "SECURITIES");
  const offered = draft.legs.length > 0 ? depots : accounts;

  const cadences: Array<SelectOption<string>> = PLAN_CADENCES.map((c) => ({
    value: `${c.unit}:${c.count}`,
    label: i18n._(c.label),
  }));
  const cadence = `${draft.interval_unit}:${draft.interval_count}`;
  const setCadence = (value: string) => {
    const [unit, count] = value.split(":");
    onChange({
      ...draft,
      interval_unit: unit as PlanInput["interval_unit"],
      interval_count: Number(count),
    });
  };

  const setLeg = (index: number, patch: Partial<PlanInput["legs"][number]>) =>
    onChange({
      ...draft,
      legs: draft.legs.map((leg, i) => (i === index ? { ...leg, ...patch } : leg)),
    });

  const addLeg = () => {
    const used = new Set(draft.legs.map((l) => l.security_id));
    const next = securities.find((s) => !used.has(s.id));
    if (!next) return;
    onChange({
      ...draft,
      // The first instrument moves the plan onto a depot: cash cannot hold a share.
      account_id: depots.some((a) => a.id === draft.account_id) ? draft.account_id : (depots[0]?.id ?? ""),
      legs: [...draft.legs, { security_id: next.id, weight: "1" }],
    });
  };

  const dropLeg = (index: number) => onChange({ ...draft, legs: draft.legs.filter((_, i) => i !== index) });

  const ready =
    draft.name.trim().length > 0 &&
    draft.account_id.length > 0 &&
    amountOf(draft.amount) > 0 &&
    amountOf(draft.fees) + amountOf(draft.taxes) < amountOf(draft.amount) &&
    draft.legs.every((leg) => amountOf(leg.weight) > 0);

  return (
    <FormDialog
      title={title ?? (draft.id ? t`Edit plan` : t`New plan`)}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
      ready={ready}
      submitLabel={submitLabel}
      wide
    >
      <Field label={t`Name`} hint={t`What you call this contribution — "Monthly savings", "Bonus"`}>
        <input
          value={draft.name}
          onChange={(e) => onChange({ ...draft, name: e.target.value })}
          placeholder={t`Monthly savings`}
        />
      </Field>

      <FieldSet label={t`How much`} hint={t`Costs are taken off before the money is split`}>
        <FieldRow note={draft.currency}>
          <input
            inputMode="decimal"
            aria-label={t`Amount`}
            placeholder={t`Amount`}
            value={draft.amount}
            onChange={(e) => onChange({ ...draft, amount: e.target.value })}
          />
          <input
            aria-label={t`Currency`}
            maxLength={3}
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </FieldRow>
        <FieldRow note={t`fee and tax`}>
          <input
            inputMode="decimal"
            aria-label={t`Commission`}
            placeholder={t`Commission`}
            value={draft.fees ?? ""}
            onChange={(e) => onChange({ ...draft, fees: e.target.value })}
          />
          <input
            inputMode="decimal"
            aria-label={t`Tax`}
            placeholder={t`Tax`}
            value={draft.taxes ?? ""}
            onChange={(e) => onChange({ ...draft, taxes: e.target.value })}
          />
        </FieldRow>
      </FieldSet>

      <Field
        label={t`Account`}
        hint={
          draft.legs.length > 0
            ? t`The securities account the purchases land on; the money comes from its cash account`
            : t`The cash account the contribution lands on`
        }
        options={offered.map((a) => ({ value: a.id, label: `${a.name} · ${a.currency}` }))}
        value={draft.account_id}
        onChange={(account_id) => onChange({ ...draft, account_id })}
      />

      <Field
        label={t`How often`}
        options={cadences}
        value={cadence}
        onChange={setCadence}
        hint={t`Counted from the first contribution, so a plan starting on the 31st keeps the 31st`}
      />

      <FieldSet label={t`Runs from`} hint={t`Leave the end empty to run until you stop the plan`}>
        <FieldRow note={t`start and end`}>
          <input
            type="date"
            aria-label={t`First contribution`}
            value={draft.start}
            onChange={(e) => onChange({ ...draft, start: e.target.value })}
          />
          <input
            type="date"
            aria-label={t`Last contribution`}
            value={draft.end ?? ""}
            onChange={(e) => onChange({ ...draft, end: e.target.value || null })}
          />
        </FieldRow>
      </FieldSet>

      <FieldSet
        label={t`What it buys`}
        hint={
          draft.legs.length === 0
            ? t`Without an instrument the contribution stays as cash on the account.`
            : t`Weights are read against each other, so 60 and 40 split the same way as 6 and 4.`
        }
      >
        {draft.legs.map((leg, index) => (
          <FieldRow
            key={leg.security_id}
            note={formatPercent(String(legShare(draft.legs, index)), { digits: 1 })}
            end={
              <button
                type="button"
                className="iconbtn iconbtn--danger"
                aria-label={t`Remove instrument`}
                onClick={() => dropLeg(index)}
              >
                <TrashIcon />
              </button>
            }
          >
            <select
              aria-label={t`Instrument`}
              value={leg.security_id}
              onChange={(e) => setLeg(index, { security_id: e.target.value })}
            >
              {securities.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.symbol} · {s.name}
                </option>
              ))}
            </select>
            <input
              inputMode="decimal"
              aria-label={t`Weight`}
              value={leg.weight}
              onChange={(e) => setLeg(index, { weight: e.target.value })}
            />
          </FieldRow>
        ))}
        <button
          type="button"
          className="btn btn--ghost"
          onClick={addLeg}
          disabled={depots.length === 0 || draft.legs.length >= securities.length}
        >
          <PlusIcon /> <Trans>Add instrument</Trans>
        </button>
      </FieldSet>

      <CheckField
        label={t`Active`}
        checked={draft.active}
        onChange={(active) => onChange({ ...draft, active })}
      />
    </FormDialog>
  );
}
