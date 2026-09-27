import { MagnifyingGlassIcon } from "@phosphor-icons/react";

/** The one search field; filtering is the caller's. */
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
        data-search=""
        aria-label={label ?? placeholder}
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
