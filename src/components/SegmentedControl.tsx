import type { ReactNode } from "react";

/**
 * Mutually exclusive choices shown side by side (theme, filters). Renders a
 * radiogroup so arrow keys and screen readers behave like native radios.
 */
export default function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: { value: T; label: ReactNode; description?: string }[];
  onChange: (next: T) => void;
  disabled?: boolean;
}) {
  const selected = options.findIndex((option) => option.value === value);
  const tabStop = selected < 0 ? 0 : selected;
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option, index) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          title={option.description}
          disabled={disabled}
          tabIndex={!disabled && index === tabStop ? 0 : -1}
          onClick={() => onChange(option.value)}
          onKeyDown={(event) => {
            if (disabled) return;
            const next =
              event.key === "ArrowRight" || event.key === "ArrowDown"
                ? (index + 1) % options.length
                : event.key === "ArrowLeft" || event.key === "ArrowUp"
                  ? (index - 1 + options.length) % options.length
                  : event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? options.length - 1
                      : -1;
            if (next < 0) return;
            event.preventDefault();
            onChange(options[next].value);
            event.currentTarget.parentElement
              ?.querySelectorAll<HTMLButtonElement>('[role="radio"]')
              [next]?.focus({ preventScroll: true });
          }}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
