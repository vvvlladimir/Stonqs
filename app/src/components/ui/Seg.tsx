import type { ReactNode } from "react";

/** Uses segmented controls for small fixed sets and a select otherwise. */

export type Option<T extends string> = {
  value: T;
  label: ReactNode;
  /** Heading the option is listed under. Options without one stay above the first group. */
  group?: string;
};

interface SegProps<T extends string> {
  options: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  label?: string;
}

/** An action that belongs to the set rather than to one option — editing it, adding to it. */
interface SegSlot {
  end?: ReactNode;
}

export function Seg<T extends string>({ options, value, onChange, label, end }: SegProps<T> & SegSlot) {
  return (
    <div className="seg" role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          aria-pressed={o.value === value}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
      {end && <span className="seg__end">{end}</span>}
    </div>
  );
}

interface ChoiceProps<T extends string> extends SegProps<T> {
  /** Whether the options are a small code-defined set. */
  fixed?: boolean;
  /** Leading empty option — the "not chosen" case. A choice that can be empty is never segmented. */
  placeholder?: string;
  disabled?: boolean;
  /** Keeps a control whose names are short from collapsing to their width. */
  wide?: boolean;
  /** Caps the width instead: a picker inside a table cell must not widen its column. */
  tight?: boolean;
}

export function Choice<T extends string>({
  options,
  value,
  onChange,
  label,
  fixed,
  placeholder,
  disabled,
  wide,
  tight,
}: ChoiceProps<T>) {
  if (fixed && placeholder === undefined && options.length <= 5) {
    return <Seg options={options} value={value} onChange={onChange} label={label} />;
  }
  return (
    <select
      aria-label={label}
      className={[wide ? "sel--wide" : "", tight ? "sel--tight" : ""].filter(Boolean).join(" ") || undefined}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value as T)}
    >
      {placeholder !== undefined && <option value="">{placeholder}</option>}
      {options
        .filter((o) => !o.group)
        .map((o) => (
          <option key={o.value} value={o.value}>
            {typeof o.label === "string" ? o.label : o.value}
          </option>
        ))}
      {[...new Set(options.map((o) => o.group).filter(Boolean))].map((group) => (
        <optgroup key={group} label={group}>
          {options
            .filter((o) => o.group === group)
            .map((o) => (
              <option key={o.value} value={o.value}>
                {typeof o.label === "string" ? o.label : o.value}
              </option>
            ))}
        </optgroup>
      ))}
    </select>
  );
}
