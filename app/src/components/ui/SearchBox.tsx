import { MagnifyingGlassIcon } from "@phosphor-icons/react";

/**
 * The one search field of the product: a magnifier and an input in a single control.
 * Filtering is always the caller's — the box only reports what was typed.
 */
export function SearchBox({
  value,
  onChange,
  placeholder,
  label,
  onEnter,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  /** Accessible name when the placeholder alone does not say what is searched. */
  label?: string;
  /** Enter runs a search that filtering as you type cannot, such as one over the network. */
  onEnter?: () => void;
}) {
  return (
    <label className="searchbox">
      <MagnifyingGlassIcon />
      <input
        type="text"
        aria-label={label}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={
          onEnter &&
          ((e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) {
              e.preventDefault();
              onEnter();
            }
          })
        }
      />
    </label>
  );
}
