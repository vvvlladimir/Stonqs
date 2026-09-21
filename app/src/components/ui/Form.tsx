import { useState, type ReactNode } from "react";
import { EyeIcon, EyeSlashIcon } from "@phosphor-icons/react";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useErrorText } from "./Async";

/**
 * Form primitives. Fields are full-width, 44px tall — the minimum touch target, not styling.
 * The wording of the buttons lives here once: a screen never writes "Saving…" itself.
 */
export const SUBMIT = msg`Save`;
export const SUBMITTING = msg`Saving…`;
export const CANCEL = msg`Cancel`;

/** `<option>` holds text only, so a select's label is a string, not a ReactNode. A disabled
 * option is shown but cannot be picked — the choice exists, something else is missing. */
export type SelectOption<T extends string> = { value: T; label: string; disabled?: boolean };

interface FieldProps<T extends string> {
  label: ReactNode;
  hint?: ReactNode;
  /** Renders the `<select>` itself; without it the control comes as `children`. */
  options?: SelectOption<T>[];
  value?: T | null;
  onChange?: (value: T) => void;
  /** Label of the leading empty option — the "not chosen" case, e.g. "— manual —". */
  placeholder?: string;
  disabled?: boolean;
  children?: ReactNode;
}

export function Field<T extends string>({
  label,
  hint,
  options,
  value,
  onChange,
  placeholder,
  disabled,
  children,
}: FieldProps<T>) {
  return (
    <label className="field">
      <span className="field__label">{label}</span>
      {options ? (
        <select value={value ?? ""} disabled={disabled} onChange={(e) => onChange?.(e.target.value as T)}>
          {placeholder !== undefined && <option value="">{placeholder}</option>}
          {options.map((o) => (
            <option key={o.value} value={o.value} disabled={o.disabled}>
              {o.label}
            </option>
          ))}
        </select>
      ) : (
        children
      )}
      {hint && <span className="field__hint">{hint}</span>}
    </label>
  );
}

interface CheckFieldProps {
  label: ReactNode;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** What ticking it actually does, where the label alone would not say. */
  hint?: ReactNode;
}

export function CheckField({ label, checked, onChange, disabled, hint }: CheckFieldProps) {
  return (
    <label className={hint ? "field field--checkbox field--checkbox--note" : "field field--checkbox"}>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span>
        {label}
        {hint && <span className="field__hint">{hint}</span>}
      </span>
    </label>
  );
}

/**
 * One repeated line inside a [`FieldSet`]: an instrument and its weight, a value and its unit.
 * Lives here rather than in a screen so every such editor is the same line.
 */
export function FieldRow({
  note,
  end,
  children,
}: {
  /** A read-only figure the row computes from what was typed: a share, a total. */
  note?: ReactNode;
  /** Trailing control that is not an input: the button that removes the row. */
  end?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="field__row">
      {children}
      {note !== undefined && <span className="field__row-note num">{note}</span>}
      {end}
    </div>
  );
}

/** A labelled group of controls — several checkboxes answering one question. */
export function FieldSet({
  label,
  hint,
  children,
}: {
  label: ReactNode;
  hint?: ReactNode;
  children: ReactNode;
}) {
  return (
    <fieldset className="field">
      <span className="field__label">{label}</span>
      {children}
      {hint && <span className="field__hint">{hint}</span>}
    </fieldset>
  );
}

/** A name and the short code that goes with it on one line: an account and its currency. */
export function FieldPair({ children }: { children: ReactNode }) {
  return <div className="field-pair">{children}</div>;
}

/** The only button row in the app. */
export function Actions({ children }: { children: ReactNode }) {
  return <div className="form__actions">{children}</div>;
}

/** The only red text of the product: a failed mutation, or a reason the screen states itself. */
export function ErrorText({
  error,
  as: Tag = "p",
  children,
}: {
  error?: Error | null;
  /** `li` inside a list, `div` around a block. */
  as?: "p" | "li" | "div";
  children?: ReactNode;
}) {
  const localized = useErrorText(error);
  const text = children ?? (error ? localized : undefined);
  if (text === undefined || text === null || text === false) return null;
  return <Tag className="err">{text}</Tag>;
}

/** Share of something, in percent: the width is the field, not the screen. */
export function PercentInput({
  value,
  onChange,
  label,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  disabled?: boolean;
}) {
  return (
    <input
      className="inp--pct"
      inputMode="decimal"
      aria-label={label}
      placeholder="0"
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

/**
 * A password or an API key: hidden by default, with an eye to show what was typed — a key pasted
 * from somewhere is worth checking before it is sealed away for good.
 */
export function SecretInput({
  value,
  onChange,
  autoComplete,
  autoFocus,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  /** `current-password`, `new-password`, or `off` for an API key. */
  autoComplete: string;
  autoFocus?: boolean;
  placeholder?: string;
}) {
  const { t } = useLingui();
  const [shown, setShown] = useState(false);
  return (
    <span className="secret">
      <input
        type={shown ? "text" : "password"}
        autoComplete={autoComplete}
        autoFocus={autoFocus}
        spellCheck={false}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <button
        type="button"
        className="iconbtn iconbtn--sm"
        aria-label={shown ? t`Hide` : t`Show`}
        aria-pressed={shown}
        onClick={() => setShown(!shown)}
      >
        {shown ? <EyeSlashIcon /> : <EyeIcon />}
      </button>
    </span>
  );
}

interface SubmitProps {
  /** Id of the form this button submits — set when the button sits outside it. */
  form?: string;
  busy?: boolean;
  ready?: boolean;
  label: string;
  busyLabel: string;
}

export function Submit({ form, busy, ready = true, label, busyLabel }: SubmitProps) {
  return (
    <button className="btn" type="submit" form={form} disabled={busy || !ready}>
      {busy ? busyLabel : label}
    </button>
  );
}

export interface FormProps {
  /** Omitted when the fields save themselves — the form then shows no button row. */
  onSubmit?: () => void;
  busy?: boolean;
  error?: Error | null;
  submitLabel?: string;
  busyLabel?: string;
  /** Blocks submit while false: an empty name, nothing edited, a sum over 100 %. */
  ready?: boolean;
  onCancel?: () => void;
  /** Extra buttons in the row, after the submit one. */
  actions?: ReactNode;
  children: ReactNode;
}

export function Form({
  onSubmit,
  busy,
  error,
  submitLabel,
  busyLabel,
  ready,
  onCancel,
  actions,
  children,
}: FormProps) {
  const { i18n } = useLingui();
  return (
    <form
      className="form"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit?.();
      }}
    >
      {children}
      {(onSubmit || onCancel || actions) && (
        <Actions>
          {onSubmit && (
            <Submit
              busy={busy}
              ready={ready}
              label={submitLabel ?? i18n._(SUBMIT)}
              busyLabel={busyLabel ?? i18n._(SUBMITTING)}
            />
          )}
          {onCancel && (
            <button className="btn btn--ghost" type="button" onClick={onCancel}>
              {i18n._(CANCEL)}
            </button>
          )}
          {actions}
        </Actions>
      )}
      <ErrorText error={error} />
    </form>
  );
}
