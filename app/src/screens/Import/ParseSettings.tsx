import { useLingui } from "@lingui/react/macro";
import { CheckField, Field, Form } from "../../components/ui";
import type { AmountBasis, AmountSign, ImportMapping, ParseConfig } from "../../lib/types";
import { BASIS_LABELS, SIGN_LABELS } from "./labels";

/** Parser settings: detected values and user overrides. */
export function ParseSettings({
  config,
  mapping,
  detected,
  detectedBasis,
  onChange,
}: {
  config: ParseConfig;
  mapping: ImportMapping;
  detected: AmountSign;
  detectedBasis: AmountBasis;
  onChange: (c: ParseConfig, m: ImportMapping) => void;
}) {
  const { t, i18n } = useLingui();
  return (
    <Form>
      <Field label={t`Delimiter`} hint={t`Detected from the content; it can be replaced`}>
        <input
          value={config.delimiter ?? ""}
          maxLength={1}
          onChange={(e) => onChange({ ...config, delimiter: e.target.value || null }, mapping)}
        />
      </Field>
      <Field label={t`Decimal mark`}>
        <input
          value={config.decimal_separator ?? ""}
          maxLength={1}
          onChange={(e) => onChange({ ...config, decimal_separator: e.target.value || null }, mapping)}
        />
      </Field>
      <Field label={t`Date format`} hint={t`In chrono terms, for example %d.%m.%Y`}>
        <input
          value={config.date_format ?? ""}
          onChange={(e) => onChange({ ...config, date_format: e.target.value || null }, mapping)}
        />
      </Field>
      <Field label={t`Rows to skip at the top`} hint={t`The broker's header above the table`}>
        <input
          inputMode="numeric"
          value={config.skip_top_rows}
          onChange={(e) => onChange({ ...config, skip_top_rows: Number(e.target.value) || 0 }, mapping)}
        />
      </Field>
      <Field label={t`Rows to drop at the bottom`} hint={t`The "Total" summary rows`}>
        <input
          inputMode="numeric"
          value={config.skip_bottom_rows}
          onChange={(e) => onChange({ ...config, skip_bottom_rows: Number(e.target.value) || 0 }, mapping)}
        />
      </Field>
      <Field
        label={t`Sign of the amount`}
        hint={
          mapping.amount_sign
            ? t`Set by hand`
            : t`Detected from the file: ${i18n._(SIGN_LABELS[detected]).toLowerCase()}`
        }
        placeholder={t`— detect from the file —`}
        options={[
          { value: "SIGNED", label: i18n._(SIGN_LABELS.SIGNED) },
          { value: "UNSIGNED", label: i18n._(SIGN_LABELS.UNSIGNED) },
        ]}
        value={mapping.amount_sign}
        onChange={(sign) =>
          onChange(config, { ...mapping, amount_sign: (sign || null) as AmountSign | null })
        }
      />
      <Field
        label={t`What the amount holds`}
        hint={
          mapping.amount_basis
            ? t`Set by hand`
            : t`Detected from the file: ${i18n._(BASIS_LABELS[detectedBasis]).toLowerCase()}`
        }
        placeholder={t`— detect from the file —`}
        options={[
          { value: "GROSS", label: i18n._(BASIS_LABELS.GROSS) },
          { value: "NET", label: i18n._(BASIS_LABELS.NET) },
        ]}
        value={mapping.amount_basis}
        onChange={(basis) =>
          onChange(config, { ...mapping, amount_basis: (basis || null) as AmountBasis | null })
        }
      />
      <CheckField
        label={t`The first row holds the headers`}
        checked={config.has_header !== false}
        onChange={(on) => onChange({ ...config, has_header: on }, mapping)}
      />
    </Form>
  );
}
