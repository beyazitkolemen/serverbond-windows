import { useEffect, useId, useRef, useState } from "react";

/**
 * Numeric input that lets the user clear the field or type a leading minus
 * without the value snapping back to 0. The parent only hears about complete
 * numbers inside [min, max]; anything else stays local and marks the field
 * invalid so the surrounding form refuses to submit.
 */
export default function NumberField({
  label,
  value,
  onChange,
  min = 1,
  max = 65535,
  step,
  disabled,
  hint,
}: {
  label: string;
  value: number;
  onChange: (n: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  hint?: string;
}) {
  const [text, setText] = useState(String(value));
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const form = input.current?.form;
    const reset = () => setText(String(value));
    form?.addEventListener("reset", reset);
    return () => form?.removeEventListener("reset", reset);
  }, [value]);
  useEffect(() => {
    // Follow outside changes (reset, saved value) unless the user is mid-edit
    // on an equivalent number such as "08" or "-".
    if (Number(text) !== value || text.trim() === "") setText(String(value));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);
  const parsed = text.trim() === "" ? NaN : Number(text);
  const invalid = !Number.isInteger(parsed) || parsed < min || parsed > max;
  return (
    <label>
      {label}
      <input
        ref={input}
        required
        type="number"
        aria-label={label}
        aria-describedby={
          [hint ? `${id}-hint` : "", invalid ? `${id}-error` : ""]
            .filter(Boolean)
            .join(" ") || undefined
        }
        inputMode="numeric"
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        value={text}
        aria-invalid={invalid || undefined}
        onChange={(e) => {
          const next = e.target.value;
          setText(next);
          const n = next.trim() === "" ? NaN : Number(next);
          if (Number.isInteger(n) && n >= min && n <= max) onChange(n);
        }}
      />
      {invalid && (
        <small id={`${id}-error`} className="field-error">
          {min}–{max} arasında bir tam sayı girin.
        </small>
      )}
      {hint ? <small id={`${id}-hint`}>{hint}</small> : null}
    </label>
  );
}
