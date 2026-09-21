import type { ReactNode } from "react";

/** One keyboard stop per tab list; arrow keys activate and focus adjacent tabs. */
export default function SectionTabs<T extends string>({
  id,
  label,
  items,
  value,
  onChange,
}: {
  id: string;
  label: string;
  items: readonly { id: T; label: string; indicator?: ReactNode }[];
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <div className="section-tabs" role="tablist" aria-label={label}>
      {items.map((item, index) => (
        <button
          key={item.id}
          id={`${id}-${item.id}`}
          type="button"
          role="tab"
          aria-selected={value === item.id}
          aria-controls={`${id}-panel`}
          tabIndex={value === item.id ? 0 : -1}
          onClick={() => onChange(item.id)}
          onKeyDown={(event) => {
            const next =
              event.key === "ArrowRight"
                ? (index + 1) % items.length
                : event.key === "ArrowLeft"
                  ? (index - 1 + items.length) % items.length
                  : event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? items.length - 1
                      : -1;
            if (next < 0) return;
            event.preventDefault();
            onChange(items[next].id);
            const button =
              event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>(
                '[role="tab"]',
              )[next];
            button?.focus({ preventScroll: true });
            button?.scrollIntoView({ block: "nearest", inline: "nearest" });
          }}
        >
          {item.label}
          {item.indicator}
        </button>
      ))}
    </div>
  );
}
