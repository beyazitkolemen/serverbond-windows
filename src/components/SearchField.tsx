import { useId, useRef } from "react";
import { Search, X } from "lucide-react";

export default function SearchField({
  value,
  onChange,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  return (
    <div className="search-field">
      <label className="visually-hidden" htmlFor={id}>
        {label}
      </label>
      <Search size={17} aria-hidden="true" />
      <input
        ref={input}
        id={id}
        type="search"
        value={value}
        placeholder={label}
        onChange={(event) => onChange(event.target.value)}
      />
      {value ? (
        <button
          type="button"
          className="icon-button"
          aria-label={`${label}: aramayı temizle`}
          onClick={() => {
            onChange("");
            input.current?.focus();
          }}
        >
          <X size={15} />
        </button>
      ) : null}
    </div>
  );
}
